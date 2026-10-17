using System.Collections.Generic;
using FluentAssertions;
using UbntDiscovery.Protocol;
using Xunit;

namespace UbntDiscovery.Tests;

public sealed class TlvReaderTests
{
    private static byte[] Tlv(byte type, params byte[] value)
    {
        var record = new List<byte> { type, (byte)(value.Length >> 8), (byte)(value.Length & 0xFF) };
        record.AddRange(value);
        return record.ToArray();
    }

    private static byte[] Packet(IEnumerable<byte[]> records)
    {
        var bytes = new List<byte> { 0x01, 0x00, 0x00, 0x00 }; // header
        foreach (var r in records)
        {
            bytes.AddRange(r);
        }

        // Fix up the declared body length in the header.
        var bodyLen = bytes.Count - 4;
        bytes[2] = (byte)(bodyLen >> 8);
        bytes[3] = (byte)(bodyLen & 0xFF);
        return bytes.ToArray();
    }

    [Fact]
    public void Parse_ExtractsCoreFields_FromWellFormedPacket()
    {
        var mac = new byte[] { 0x00, 0x15, 0x6D, 0x01, 0x02, 0x03 };
        var ip = new byte[] { 192, 168, 1, 20 };
        var packet = Packet(new[]
        {
            Tlv(0x02, Concat(mac, ip)),                 // MAC + IP
            Tlv(0x03, Ascii("WA.v8.5.5")),              // firmware
            Tlv(0x0A, 0x00, 0x01, 0x00, 0x00),          // uptime 65536s
            Tlv(0x0B, Ascii("rocket-m5")),              // hostname
            Tlv(0x0C, Ascii("N5N")),                    // model code
            Tlv(0x0D, Ascii("my-wlan")),                // ESSID
        });

        var device = TlvReader.Parse(packet);

        device.Should().NotBeNull();
        device!.Mac.Should().Be("00:15:6D:01:02:03");
        device.Ip!.ToString().Should().Be("192.168.1.20");
        device.Firmware.Should().Be("WA.v8.5.5");
        device.Hostname.Should().Be("rocket-m5");
        device.ModelCode.Should().Be("N5N");
        device.ProductName.Should().Be("NanoStation M5");
        device.Essid.Should().Be("my-wlan");
        device.UptimeSeconds.Should().Be(65536u);
        device.Addresses.Should().ContainSingle();
    }

    [Fact]
    public void Parse_UsesMacOnlyTag_WhenNoAddressPairPresent()
    {
        var mac = new byte[] { 0xDC, 0x9F, 0xDB, 0xAA, 0xBB, 0xCC };
        var packet = Packet(new[] { Tlv(0x01, mac), Tlv(0x0C, Ascii("BZ2")) });

        var device = TlvReader.Parse(packet);

        device.Should().NotBeNull();
        device!.Mac.Should().Be("DC:9F:DB:AA:BB:CC");
        device.ProductName.Should().Be("UniFi AP");
        device.Ip.Should().BeNull();
    }

    [Fact]
    public void Parse_ReturnsNull_WhenPacketTooShort()
    {
        TlvReader.Parse(new byte[] { 0x01, 0x00 }).Should().BeNull();
    }

    [Fact]
    public void Parse_ReturnsNull_WhenNoMacPresent()
    {
        var packet = Packet(new[] { Tlv(0x03, Ascii("firmware-only")) });
        TlvReader.Parse(packet).Should().BeNull();
    }

    [Fact]
    public void Parse_DoesNotThrow_OnTruncatedLength()
    {
        // Declares a 200-byte value but supplies far fewer bytes.
        var mac = new byte[] { 0x00, 0x11, 0x22, 0x33, 0x44, 0x55 };
        var good = Packet(new[] { Tlv(0x02, Concat(mac, new byte[] { 10, 0, 0, 5 })) });

        // Append a bogus record claiming a huge length.
        var malformed = new List<byte>(good) { 0x0B, 0xFF, 0xFF, 0x41, 0x42 };

        var act = () => TlvReader.Parse(malformed.ToArray());

        act.Should().NotThrow();
        // The valid MAC/IP record before the bad one is still recovered.
        var device = TlvReader.Parse(malformed.ToArray());
        device.Should().NotBeNull();
        device!.Mac.Should().Be("00:11:22:33:44:55");
    }

    [Fact]
    public void Parse_SkipsUnknownTags()
    {
        var mac = new byte[] { 0x01, 0x02, 0x03, 0x04, 0x05, 0x06 };
        var packet = Packet(new[]
        {
            Tlv(0x7F, 0xDE, 0xAD, 0xBE, 0xEF), // unknown tag
            Tlv(0x01, mac),
        });

        var device = TlvReader.Parse(packet);

        device.Should().NotBeNull();
        device!.Mac.Should().Be("01:02:03:04:05:06");
    }

    private static byte[] Ascii(string s) => System.Text.Encoding.ASCII.GetBytes(s);

    private static byte[] Concat(byte[] a, byte[] b)
    {
        var result = new byte[a.Length + b.Length];
        a.CopyTo(result, 0);
        b.CopyTo(result, a.Length);
        return result;
    }
}
