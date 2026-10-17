using FluentAssertions;
using UbntDiscovery.Protocol;
using Xunit;

namespace UbntDiscovery.Tests;

public sealed class ModelNamesTests
{
    [Theory]
    [InlineData("N5N", "NanoStation M5")]
    [InlineData("BZ2", "UniFi AP")]
    [InlineData("ERLite-3", "EdgeRouter Lite")]
    [InlineData("R5N", "Rocket M5")]
    [InlineData("n5n", "NanoStation M5")] // case-insensitive
    public void Resolve_MapsKnownCodes(string code, string expected)
    {
        ModelNames.Resolve(code).Should().Be(expected);
    }

    [Fact]
    public void Resolve_ReturnsCodeItself_WhenUnknown()
    {
        ModelNames.Resolve("ZZZ").Should().Be("ZZZ");
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("   ")]
    public void Resolve_ReturnsNull_ForBlank(string? code)
    {
        ModelNames.Resolve(code).Should().BeNull();
    }
}
