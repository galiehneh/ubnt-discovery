using System.Collections.Concurrent;
using System.Collections.Immutable;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;

namespace UbntDiscovery.Protocol;

/// <summary>
/// Sends UDP discovery probes on every usable local IPv4 interface and collects
/// the replies, de-duplicating devices by MAC and merging new field values into
/// the existing snapshot.
/// </summary>
public sealed class DiscoveryClient : IDisposable
{
    private static readonly IPEndPoint GlobalBroadcast = new(IPAddress.Broadcast, DiscoveryRequest.Port);

    private readonly ConcurrentDictionary<string, DiscoveredDevice> _devices = new(StringComparer.OrdinalIgnoreCase);
    private readonly List<UdpClient> _sockets = new();
    private readonly object _mergeLock = new();

    /// <summary>Raised (on a background thread) whenever a device is found or updated.</summary>
    public event EventHandler<DiscoveredDevice>? DeviceDiscovered;

    /// <summary>
    /// Runs one scan: opens sockets, broadcasts probes periodically, and receives
    /// replies until <paramref name="cancellationToken"/> is cancelled (the caller
    /// controls the scan duration). Returns the accumulated device set.
    /// </summary>
    public async Task<IReadOnlyCollection<DiscoveredDevice>> ScanAsync(CancellationToken cancellationToken)
    {
        var targets = BuildSenders();
        var receiveLoops = new List<Task>();

        foreach (var (socket, subnetBroadcast) in targets)
        {
            _sockets.Add(socket);
            receiveLoops.Add(ReceiveLoopAsync(socket, cancellationToken));
        }

        // Best-effort shared listener on the discovery port (some devices reply there).
        var portListener = TryCreatePortListener();
        if (portListener is not null)
        {
            _sockets.Add(portListener);
            receiveLoops.Add(ReceiveLoopAsync(portListener, cancellationToken));
        }

        var sendLoop = SendLoopAsync(targets, cancellationToken);

        try
        {
            await sendLoop.ConfigureAwait(false);
            await Task.WhenAll(receiveLoops).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            // Expected when the scan window elapses.
        }

        return _devices.Values.ToArray();
    }

    private static IPAddress? ComputeSubnetBroadcast(IPAddress address, IPAddress mask)
    {
        var addrBytes = address.GetAddressBytes();
        var maskBytes = mask.GetAddressBytes();
        if (addrBytes.Length != maskBytes.Length)
        {
            return null;
        }

        var broadcast = new byte[addrBytes.Length];
        for (var i = 0; i < addrBytes.Length; i++)
        {
            broadcast[i] = (byte)(addrBytes[i] | (byte)~maskBytes[i]);
        }

        return new IPAddress(broadcast);
    }

    private List<(UdpClient Socket, IPAddress? SubnetBroadcast)> BuildSenders()
    {
        var senders = new List<(UdpClient, IPAddress?)>();

        foreach (var nic in NetworkInterface.GetAllNetworkInterfaces())
        {
            if (nic.OperationalStatus != OperationalStatus.Up ||
                nic.NetworkInterfaceType == NetworkInterfaceType.Loopback)
            {
                continue;
            }

            foreach (var info in nic.GetIPProperties().UnicastAddresses)
            {
                if (info.Address.AddressFamily != AddressFamily.InterNetwork)
                {
                    continue;
                }

                try
                {
                    var socket = new UdpClient(AddressFamily.InterNetwork);
                    socket.Client.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
                    socket.Client.Bind(new IPEndPoint(info.Address, 0));
                    socket.EnableBroadcast = true;

                    var subnet = ComputeSubnetBroadcast(info.Address, info.IPv4Mask);
                    senders.Add((socket, subnet));
                }
                catch (SocketException)
                {
                    // Skip interfaces we cannot bind to.
                }
            }
        }

        return senders;
    }

    private static UdpClient? TryCreatePortListener()
    {
        try
        {
            var listener = new UdpClient(AddressFamily.InterNetwork);
            listener.Client.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
            listener.Client.Bind(new IPEndPoint(IPAddress.Any, DiscoveryRequest.Port));
            listener.EnableBroadcast = true;
            return listener;
        }
        catch (SocketException)
        {
            // Port in use (e.g. the original tool is running) — rely on sender sockets.
            return null;
        }
    }

    private static async Task SendLoopAsync(
        IReadOnlyList<(UdpClient Socket, IPAddress? SubnetBroadcast)> targets,
        CancellationToken cancellationToken)
    {
        // Re-broadcast a few times across the scan window to catch devices that miss the first probe.
        while (!cancellationToken.IsCancellationRequested)
        {
            foreach (var (socket, subnet) in targets)
            {
                foreach (var probe in DiscoveryRequest.All)
                {
                    await TrySendAsync(socket, probe, GlobalBroadcast).ConfigureAwait(false);
                    if (subnet is not null)
                    {
                        await TrySendAsync(socket, probe, new IPEndPoint(subnet, DiscoveryRequest.Port))
                            .ConfigureAwait(false);
                    }
                }
            }

            try
            {
                await Task.Delay(TimeSpan.FromSeconds(1), cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                return;
            }
        }
    }

    private static async Task TrySendAsync(UdpClient socket, byte[] probe, IPEndPoint target)
    {
        try
        {
            await socket.SendAsync(probe, probe.Length, target).ConfigureAwait(false);
        }
        catch (SocketException)
        {
            // Ignore per-target send failures (e.g. unreachable broadcast on some adapters).
        }
        catch (ObjectDisposedException)
        {
            // Socket closed during shutdown.
        }
    }

    private async Task ReceiveLoopAsync(UdpClient socket, CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            UdpReceiveResult result;
            try
            {
                result = await socket.ReceiveAsync(cancellationToken).ConfigureAwait(false);
            }
            catch (OperationCanceledException)
            {
                return;
            }
            catch (SocketException)
            {
                // Transient receive error; keep listening until cancelled.
                continue;
            }
            catch (ObjectDisposedException)
            {
                return;
            }

            var parsed = TlvReader.Parse(result.Buffer);
            if (parsed is null)
            {
                continue;
            }

            // Fill in the sender IP if the packet carried none.
            if (parsed.Ip is null && result.RemoteEndPoint.Address.AddressFamily == AddressFamily.InterNetwork)
            {
                parsed = parsed with { Ip = result.RemoteEndPoint.Address };
            }

            var merged = Merge(parsed);
            DeviceDiscovered?.Invoke(this, merged);
        }
    }

    private DiscoveredDevice Merge(DiscoveredDevice incoming)
    {
        lock (_mergeLock)
        {
            if (!_devices.TryGetValue(incoming.Mac, out var existing))
            {
                _devices[incoming.Mac] = incoming;
                return incoming;
            }

            var addresses = existing.Addresses
                .Concat(incoming.Addresses)
                .GroupBy(a => (a.Mac, a.Ip?.ToString()))
                .Select(g => g.First())
                .ToImmutableArray();

            var merged = existing with
            {
                Ip = incoming.Ip ?? existing.Ip,
                Netmask = incoming.Netmask ?? existing.Netmask,
                Hostname = Prefer(incoming.Hostname, existing.Hostname),
                Essid = Prefer(incoming.Essid, existing.Essid),
                Firmware = Prefer(incoming.Firmware, existing.Firmware),
                ModelCode = Prefer(incoming.ModelCode, existing.ModelCode),
                ProductName = Prefer(incoming.ProductName, existing.ProductName),
                UptimeSeconds = incoming.UptimeSeconds ?? existing.UptimeSeconds,
                Addresses = addresses,
                LastSeen = incoming.LastSeen,
            };

            _devices[incoming.Mac] = merged;
            return merged;
        }
    }

    private static string? Prefer(string? incoming, string? existing) =>
        !string.IsNullOrWhiteSpace(incoming) ? incoming : existing;

    public void Dispose()
    {
        foreach (var socket in _sockets)
        {
            try
            {
                socket.Dispose();
            }
            catch
            {
                // Ignore disposal races.
            }
        }

        _sockets.Clear();
    }
}
