namespace UbntDiscovery.Protocol;

/// <summary>
/// Builds the UDP discovery request payloads broadcast to port 10001.
/// </summary>
public static class DiscoveryRequest
{
    /// <summary>The Ubiquiti UDP discovery port.</summary>
    public const int Port = 10001;

    /// <summary>
    /// Version-1 discovery probe: <c>01 00 00 00</c> — version 1, command 0
    /// ("get all"), zero-length payload. This is what the original tool and
    /// airOS devices answer to.
    /// </summary>
    public static byte[] V1 { get; } = { 0x01, 0x00, 0x00, 0x00 };

    /// <summary>
    /// Version-2 discovery probe: <c>02 06 00 00</c>. Newer firmwares reply to
    /// this as well; sending both widens device coverage.
    /// </summary>
    public static byte[] V2 { get; } = { 0x02, 0x06, 0x00, 0x00 };

    /// <summary>All probe payloads to send per scan.</summary>
    public static IReadOnlyList<byte[]> All { get; } = new[] { V1, V2 };
}
