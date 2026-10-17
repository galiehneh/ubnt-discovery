using System.Collections.Immutable;
using System.Net;

namespace UbntDiscovery.Protocol;

/// <summary>
/// One network address pair reported by a device (a device may report several,
/// e.g. one per radio/VLAN). Mirrors the "Device Network Addresses" detail view.
/// </summary>
public sealed record DeviceAddress(string Mac, IPAddress? Ip);

/// <summary>
/// Immutable snapshot of a device discovered on the LAN. New instances are
/// produced on every update rather than mutating in place.
/// </summary>
public sealed record DiscoveredDevice
{
    /// <summary>Primary hardware (MAC) address — the identity key for de-duplication.</summary>
    public required string Mac { get; init; }

    public IPAddress? Ip { get; init; }
    public string? Netmask { get; init; }
    public string? Hostname { get; init; }
    public string? Essid { get; init; }
    public string? Firmware { get; init; }

    /// <summary>Short platform code reported by the device, e.g. "N5N".</summary>
    public string? ModelCode { get; init; }

    /// <summary>Full product name resolved from <see cref="ModelCode"/>, e.g. "NanoStation M5".</summary>
    public string? ProductName { get; init; }

    public uint? UptimeSeconds { get; init; }

    /// <summary>All address pairs reported by the device.</summary>
    public ImmutableArray<DeviceAddress> Addresses { get; init; } = ImmutableArray<DeviceAddress>.Empty;

    /// <summary>When this device was last heard from.</summary>
    public DateTimeOffset LastSeen { get; init; } = DateTimeOffset.Now;

    /// <summary>Best human-readable product label (falls back to the raw code, then "Unknown").</summary>
    public string DisplayProduct =>
        !string.IsNullOrWhiteSpace(ProductName) ? ProductName
        : !string.IsNullOrWhiteSpace(ModelCode) ? ModelCode
        : "Unknown";

    public string DisplayIp => Ip?.ToString() ?? "";

    public string DisplayUptime => UptimeSeconds is { } s ? FormatUptime(s) : "";

    private static string FormatUptime(uint totalSeconds)
    {
        var days = totalSeconds / 86400;
        var hms = TimeSpan.FromSeconds(totalSeconds % 86400);
        var clock = $"{hms.Hours:00}:{hms.Minutes:00}:{hms.Seconds:00}";
        return days switch
        {
            0 => clock,
            1 => $"1 day {clock}",
            _ => $"{days} days {clock}",
        };
    }
}
