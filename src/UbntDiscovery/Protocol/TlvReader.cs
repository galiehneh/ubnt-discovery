using System.Collections.Immutable;
using System.Net;
using System.Text;

namespace UbntDiscovery.Protocol;

/// <summary>
/// Parses a Ubiquiti discovery reply into a <see cref="DiscoveredDevice"/>.
///
/// Wire format: a 4-byte header <c>[version][cmd][len_hi][len_lo]</c> followed by
/// a sequence of TLV records <c>[type:1][length:2 big-endian][value:length]</c>.
/// The parser is deliberately defensive: unknown tags are skipped and any
/// out-of-bounds length terminates parsing instead of throwing, so a malformed
/// or truncated packet never crashes a scan.
/// </summary>
public static class TlvReader
{
    // Known field tags (airOS discovery v1).
    private const byte TagMac = 0x01;        // 6 bytes
    private const byte TagMacIp = 0x02;      // 6-byte MAC + 4-byte IPv4
    private const byte TagFirmware = 0x03;   // string
    private const byte TagUptime = 0x0A;     // uint32 seconds
    private const byte TagHostname = 0x0B;   // string
    private const byte TagModelCode = 0x0C;  // string, e.g. "N5N"
    private const byte TagEssid = 0x0D;      // string
    private const byte TagProductName = 0x14; // string, e.g. "NanoStation M5"

    /// <summary>
    /// Parses <paramref name="data"/> (the raw UDP datagram payload). Returns
    /// <c>null</c> when the packet is too short or yields no usable MAC address.
    /// </summary>
    public static DiscoveredDevice? Parse(ReadOnlySpan<byte> data)
    {
        // Need at least the 4-byte header.
        if (data.Length < 4)
        {
            return null;
        }

        string? macFromTag = null;
        string? firmware = null;
        string? hostname = null;
        string? modelCode = null;
        string? essid = null;
        uint? uptime = null;
        var addresses = ImmutableArray.CreateBuilder<DeviceAddress>();

        var pos = 4; // skip header
        while (pos + 3 <= data.Length)
        {
            var type = data[pos];
            var length = (data[pos + 1] << 8) | data[pos + 2];
            pos += 3;

            // Bounds check: a bad length ends parsing with whatever we have.
            if (length < 0 || pos + length > data.Length)
            {
                break;
            }

            var value = data.Slice(pos, length);
            pos += length;

            switch (type)
            {
                case TagMac when length >= 6:
                    macFromTag = FormatMac(value[..6]);
                    break;

                case TagMacIp when length >= 10:
                    var mac = FormatMac(value[..6]);
                    var ip = new IPAddress(value.Slice(6, 4).ToArray());
                    addresses.Add(new DeviceAddress(mac, ip));
                    break;

                case TagFirmware:
                    firmware = DecodeString(value);
                    break;

                case TagUptime when length >= 4:
                    uptime = ((uint)value[0] << 24) | ((uint)value[1] << 16)
                        | ((uint)value[2] << 8) | value[3];
                    break;

                case TagHostname:
                    hostname = DecodeString(value);
                    break;

                case TagModelCode:
                    modelCode = DecodeString(value);
                    break;

                case TagEssid:
                    essid = DecodeString(value);
                    break;

                case TagProductName:
                    // Prefer the device-supplied name only if we can't resolve the code later.
                    var product = DecodeString(value);
                    if (!string.IsNullOrWhiteSpace(product) && string.IsNullOrWhiteSpace(modelCode))
                    {
                        modelCode = product;
                    }

                    break;

                // Unknown tags are intentionally ignored.
            }
        }

        // Determine the identity MAC and primary IP.
        var primaryMac = macFromTag ?? (addresses.Count > 0 ? addresses[0].Mac : null);
        if (string.IsNullOrEmpty(primaryMac))
        {
            return null;
        }

        var primaryIp = addresses.FirstOrDefault(a => a.Ip is not null)?.Ip;

        return new DiscoveredDevice
        {
            Mac = primaryMac,
            Ip = primaryIp,
            Firmware = firmware,
            Hostname = hostname,
            Essid = essid,
            ModelCode = modelCode,
            ProductName = ModelNames.Resolve(modelCode),
            UptimeSeconds = uptime,
            Addresses = addresses.ToImmutable(),
        };
    }

    private static string FormatMac(ReadOnlySpan<byte> mac)
    {
        var sb = new StringBuilder(17);
        for (var i = 0; i < mac.Length; i++)
        {
            if (i > 0)
            {
                sb.Append(':');
            }

            sb.Append(mac[i].ToString("X2"));
        }

        return sb.ToString();
    }

    private static string DecodeString(ReadOnlySpan<byte> value)
    {
        // Device strings are latin-1/ASCII; trim trailing NULs and whitespace.
        var text = Encoding.Latin1.GetString(value);
        return text.TrimEnd('\0').Trim();
    }
}
