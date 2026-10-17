using System.Diagnostics;
using System.Text;
using UbntDiscovery.Protocol;

namespace UbntDiscovery.Ui;

/// <summary>
/// Main window: a device table plus Scan / Clear / Details / Copy / Web UI
/// actions and a search filter — mirroring the original Discovery tool layout.
/// </summary>
public sealed class MainForm : Form
{
    private const int ScanSeconds = 6;

    private readonly Dictionary<string, DiscoveredDevice> _devices = new(StringComparer.OrdinalIgnoreCase);
    private readonly object _devicesLock = new();

    private readonly DataGridView _grid = new();
    private readonly TextBox _search = new();
    private readonly Button _scanButton = new();
    private readonly Button _clearButton = new();
    private readonly Button _detailsButton = new();
    private readonly Button _copyButton = new();
    private readonly Button _webUiButton = new();
    private readonly Label _totalLabel = new();
    private readonly System.Windows.Forms.Timer _scanTimer = new();

    private DiscoveryClient? _client;
    private CancellationTokenSource? _scanCts;
    private int _scanElapsed;

    public MainForm()
    {
        Text = "LAN Device Discovery";
        Width = 920;
        Height = 560;
        StartPosition = FormStartPosition.CenterScreen;
        MinimumSize = new Size(640, 360);

        BuildLayout();
        _scanTimer.Interval = 1000;
        _scanTimer.Tick += OnScanTick;
        UpdateActionButtons();
    }

    private void BuildLayout()
    {
        var toolbar = new FlowLayoutPanel
        {
            Dock = DockStyle.Top,
            Height = 44,
            Padding = new Padding(8, 8, 8, 4),
            FlowDirection = FlowDirection.LeftToRight,
            WrapContents = false,
        };

        ConfigureButton(_scanButton, "Scan", OnScanClicked);
        ConfigureButton(_clearButton, "Clear", (_, _) => ClearDevices());
        ConfigureButton(_detailsButton, "Details...", (_, _) => ShowDetails());
        ConfigureButton(_copyButton, "Copy", (_, _) => CopySelection());
        ConfigureButton(_webUiButton, "Web UI", (_, _) => OpenWebUi());

        var searchLabel = new Label
        {
            Text = "Search:",
            AutoSize = true,
            Anchor = AnchorStyles.Left,
            Padding = new Padding(16, 8, 4, 0),
        };
        _search.Width = 200;
        _search.TextChanged += (_, _) => ApplyFilter();

        toolbar.Controls.AddRange(new Control[]
        {
            _scanButton, _clearButton, _detailsButton, _copyButton, _webUiButton, searchLabel, _search,
        });

        _grid.Dock = DockStyle.Fill;
        _grid.ReadOnly = true;
        _grid.AllowUserToAddRows = false;
        _grid.AllowUserToDeleteRows = false;
        _grid.AllowUserToResizeRows = false;
        _grid.RowHeadersVisible = false;
        _grid.SelectionMode = DataGridViewSelectionMode.FullRowSelect;
        _grid.MultiSelect = true;
        _grid.AutoSizeColumnsMode = DataGridViewAutoSizeColumnsMode.Fill;
        _grid.BackgroundColor = SystemColors.Window;
        _grid.ColumnHeadersHeightSizeMode = DataGridViewColumnHeadersHeightSizeMode.DisableResizing;

        AddColumn("Product Name", 160);
        AddColumn("IP Address", 110);
        AddColumn("Hardware Address", 130);
        AddColumn("System Name", 140);
        AddColumn("SSID", 110);
        AddColumn("Firmware Version", 170);

        _grid.CellDoubleClick += (_, e) => { if (e.RowIndex >= 0) ShowDetails(); };
        _grid.SelectionChanged += (_, _) => UpdateActionButtons();

        var status = new StatusStrip();
        var totalItem = new ToolStripControlHost(_totalLabel) { Alignment = ToolStripItemAlignment.Right };
        _totalLabel.Text = "Total: 0";
        _totalLabel.AutoSize = true;
        status.Items.Add(totalItem);

        Controls.Add(_grid);
        Controls.Add(toolbar);
        Controls.Add(status);
    }

    private static void ConfigureButton(Button button, string text, EventHandler onClick)
    {
        button.Text = text;
        button.AutoSize = true;
        button.Margin = new Padding(0, 0, 6, 0);
        button.Padding = new Padding(8, 2, 8, 2);
        button.Click += onClick;
    }

    private void AddColumn(string header, int width)
    {
        _grid.Columns.Add(new DataGridViewTextBoxColumn
        {
            HeaderText = header,
            FillWeight = width,
            SortMode = DataGridViewColumnSortMode.Automatic,
        });
    }

    private async void OnScanClicked(object? sender, EventArgs e)
    {
        if (_scanCts is not null)
        {
            // A scan is running — cancel it early.
            _scanCts.Cancel();
            return;
        }

        _scanCts = new CancellationTokenSource();
        _client = new DiscoveryClient();
        _client.DeviceDiscovered += OnDeviceDiscovered;

        _scanElapsed = 0;
        _scanTimer.Start();
        _scanButton.Text = $"Scanning ({ScanSeconds}s)";
        UpdateActionButtons();

        try
        {
            _scanCts.CancelAfter(TimeSpan.FromSeconds(ScanSeconds));
            await _client.ScanAsync(_scanCts.Token).ConfigureAwait(true);
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, $"Scan failed: {ex.Message}", "Discovery",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
        finally
        {
            EndScan();
        }
    }

    private void EndScan()
    {
        _scanTimer.Stop();
        if (_client is not null)
        {
            _client.DeviceDiscovered -= OnDeviceDiscovered;
            _client.Dispose();
            _client = null;
        }

        _scanCts?.Dispose();
        _scanCts = null;
        _scanButton.Text = "Scan";
        UpdateActionButtons();
    }

    private void OnScanTick(object? sender, EventArgs e)
    {
        _scanElapsed++;
        var remaining = Math.Max(0, ScanSeconds - _scanElapsed);
        _scanButton.Text = $"Scanning ({remaining}s)";
    }

    private void OnDeviceDiscovered(object? sender, DiscoveredDevice device)
    {
        if (IsDisposed || !IsHandleCreated)
        {
            return;
        }

        BeginInvoke(() =>
        {
            lock (_devicesLock)
            {
                _devices[device.Mac] = device;
            }

            ApplyFilter();
        });
    }

    private void ClearDevices()
    {
        lock (_devicesLock)
        {
            _devices.Clear();
        }

        ApplyFilter();
    }

    private void ApplyFilter()
    {
        List<DiscoveredDevice> snapshot;
        lock (_devicesLock)
        {
            snapshot = _devices.Values.ToList();
        }

        var query = _search.Text.Trim();
        var filtered = string.IsNullOrEmpty(query)
            ? snapshot
            : snapshot.Where(d => MatchesSearch(d, query)).ToList();

        filtered.Sort((a, b) => string.Compare(a.DisplayProduct, b.DisplayProduct, StringComparison.OrdinalIgnoreCase));

        // Preserve selection by MAC across rebuilds.
        var selectedMac = SelectedDevice()?.Mac;

        _grid.SuspendLayout();
        _grid.Rows.Clear();
        foreach (var device in filtered)
        {
            var index = _grid.Rows.Add(
                device.DisplayProduct,
                device.DisplayIp,
                device.Mac,
                device.Hostname ?? "",
                device.Essid ?? "",
                device.Firmware ?? "");
            _grid.Rows[index].Tag = device;

            if (device.Mac.Equals(selectedMac, StringComparison.OrdinalIgnoreCase))
            {
                _grid.Rows[index].Selected = true;
            }
        }

        _grid.ResumeLayout();
        _totalLabel.Text = $"Total: {filtered.Count}";
        UpdateActionButtons();
    }

    private static bool MatchesSearch(DiscoveredDevice d, string query)
    {
        var haystack = string.Join('\u0001', new[]
        {
            d.DisplayProduct, d.DisplayIp, d.Mac, d.Hostname, d.Essid, d.Firmware, d.ModelCode,
        });
        return haystack.Contains(query, StringComparison.OrdinalIgnoreCase);
    }

    private DiscoveredDevice? SelectedDevice()
    {
        if (_grid.SelectedRows.Count > 0)
        {
            return _grid.SelectedRows[0].Tag as DiscoveredDevice;
        }

        return _grid.CurrentRow?.Tag as DiscoveredDevice;
    }

    private void UpdateActionButtons()
    {
        var hasSelection = SelectedDevice() is not null;
        _detailsButton.Enabled = hasSelection;
        _copyButton.Enabled = _grid.SelectedRows.Count > 0;
        _webUiButton.Enabled = SelectedDevice()?.Ip is not null;
        _clearButton.Enabled = _grid.Rows.Count > 0;
    }

    private void ShowDetails()
    {
        var device = SelectedDevice();
        if (device is null)
        {
            return;
        }

        using var dialog = new DetailsForm(device);
        dialog.ShowDialog(this);
    }

    private void CopySelection()
    {
        if (_grid.SelectedRows.Count == 0)
        {
            return;
        }

        var sb = new StringBuilder();
        var rows = _grid.SelectedRows.Cast<DataGridViewRow>()
            .OrderBy(r => r.Index);

        foreach (var row in rows)
        {
            var cells = row.Cells.Cast<DataGridViewCell>().Select(c => c.Value?.ToString() ?? "");
            sb.AppendLine(string.Join('\t', cells));
        }

        try
        {
            Clipboard.SetText(sb.ToString());
        }
        catch (Exception ex)
        {
            MessageBox.Show(this, $"Copy failed: {ex.Message}", "Discovery",
                MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
    }

    private void OpenWebUi()
    {
        var device = SelectedDevice();
        if (device?.Ip is null)
        {
            return;
        }

        var url = $"http://{device.Ip}";
        try
        {
            Process.Start(new ProcessStartInfo(url) { UseShellExecute = true });
        }
        catch (Exception)
        {
            MessageBox.Show(this, $"Failed launching web browser. Please open {url} manually.",
                "Discovery", MessageBoxButtons.OK, MessageBoxIcon.Warning);
        }
    }

    protected override void OnFormClosing(FormClosingEventArgs e)
    {
        _scanCts?.Cancel();
        EndScan();
        base.OnFormClosing(e);
    }
}
