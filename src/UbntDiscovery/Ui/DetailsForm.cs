using UbntDiscovery.Protocol;

namespace UbntDiscovery.Ui;

/// <summary>
/// Read-only device detail dialog: key fields plus the "Device Network Addresses"
/// table listing every MAC/IP pair the device reported.
/// </summary>
public sealed class DetailsForm : Form
{
    public DetailsForm(DiscoveredDevice device)
    {
        ArgumentNullException.ThrowIfNull(device);

        Text = $"Device Details — {device.DisplayProduct}";
        Width = 460;
        Height = 420;
        StartPosition = FormStartPosition.CenterParent;
        MinimizeBox = false;
        MaximizeBox = false;
        FormBorderStyle = FormBorderStyle.Sizable;

        var fields = new TableLayoutPanel
        {
            Dock = DockStyle.Top,
            ColumnCount = 2,
            AutoSize = true,
            AutoSizeMode = AutoSizeMode.GrowAndShrink,
            Padding = new Padding(12, 12, 12, 6),
        };
        fields.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 130));
        fields.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));

        AddField(fields, "Product Name:", device.DisplayProduct);
        AddField(fields, "HW Address:", device.Mac);
        AddField(fields, "IP Address:", device.DisplayIp);
        AddField(fields, "System Name:", device.Hostname ?? "Unknown");
        AddField(fields, "SSID:", device.Essid ?? "Unknown");
        AddField(fields, "Version:", device.Firmware ?? "Unknown");
        AddField(fields, "Uptime:", string.IsNullOrEmpty(device.DisplayUptime) ? "Unknown" : device.DisplayUptime);

        var addressesLabel = new Label
        {
            Text = "Device Network Addresses",
            Dock = DockStyle.Top,
            Padding = new Padding(12, 6, 12, 4),
            Font = new Font(Font, FontStyle.Bold),
            AutoSize = true,
        };

        var addressGrid = new DataGridView
        {
            Dock = DockStyle.Fill,
            ReadOnly = true,
            AllowUserToAddRows = false,
            RowHeadersVisible = false,
            AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill,
            SelectionMode = DataGridViewSelectionMode.FullRowSelect,
            BackgroundColor = SystemColors.Window,
        };
        addressGrid.Columns.Add(new DataGridViewTextBoxColumn { HeaderText = "Hardware Address", FillWeight = 55 });
        addressGrid.Columns.Add(new DataGridViewTextBoxColumn { HeaderText = "IP Address", FillWeight = 45 });

        IEnumerable<DeviceAddress> addresses = device.Addresses.Length > 0
            ? device.Addresses
            : new[] { new DeviceAddress(device.Mac, device.Ip) };

        foreach (var address in addresses)
        {
            addressGrid.Rows.Add(address.Mac, address.Ip?.ToString() ?? "");
        }

        var closeButton = new Button
        {
            Text = "Close",
            Dock = DockStyle.Bottom,
            Height = 32,
            Margin = new Padding(12),
        };
        closeButton.Click += (_, _) => Close();
        AcceptButton = closeButton;

        var buttonPanel = new Panel { Dock = DockStyle.Bottom, Height = 44, Padding = new Padding(12, 6, 12, 10) };
        buttonPanel.Controls.Add(closeButton);

        Controls.Add(addressGrid);
        Controls.Add(addressesLabel);
        Controls.Add(fields);
        Controls.Add(buttonPanel);
    }

    private static void AddField(TableLayoutPanel panel, string label, string value)
    {
        var row = panel.RowCount++;
        panel.Controls.Add(new Label
        {
            Text = label,
            AutoSize = true,
            Anchor = AnchorStyles.Left,
            Font = new Font(panel.Font, FontStyle.Bold),
            Margin = new Padding(0, 4, 8, 4),
        }, 0, row);

        panel.Controls.Add(new TextBox
        {
            Text = value,
            ReadOnly = true,
            BorderStyle = BorderStyle.None,
            BackColor = SystemColors.Control,
            Dock = DockStyle.Fill,
            Margin = new Padding(0, 4, 0, 4),
        }, 1, row);
    }
}
