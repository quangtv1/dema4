// Spike S4: chuẩn so sánh.
// Chép nguyên logic ExcelReader.Read của bản .NET 1.4.0
// (Kytl-main/ktl.be/ktl.be.infrastructure.external/Documents/Implements/ExcelReader.cs)
// rồi xuất JSON, kèm kiểu dữ liệu và định dạng số của từng ô để phân tích chỗ lệch.
//
// Cách dùng: closedxml-ref <vùng văn hoá | -> <file JSON kết quả> <file.xlsx>...

using System.Globalization;
using System.Text.Encodings.Web;
using System.Text.Json;
using ClosedXML.Excel;

if (args.Length < 3)
{
    Console.Error.WriteLine("Cách dùng: closedxml-ref <vùng văn hoá | -> <out.json> <file.xlsx>...");
    return 2;
}

if (args[0] != "-")
{
    var culture = CultureInfo.GetCultureInfo(args[0]);
    CultureInfo.CurrentCulture = culture;
    CultureInfo.CurrentUICulture = culture;
    CultureInfo.DefaultThreadCurrentCulture = culture;
}

var files = args.Skip(2).Select(ReadFile).ToList();
var json = JsonSerializer.Serialize(
    new { reader = "ClosedXML 0.105.1", culture = CultureInfo.CurrentCulture.Name, files },
    new JsonSerializerOptions { WriteIndented = true, Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping });
File.WriteAllText(args[1], json);
Console.WriteLine($"Đã ghi {args[1]} ({files.Count} file, vùng văn hoá {CultureInfo.CurrentCulture.Name}).");
return 0;

static object ReadFile(string path)
{
    using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);
    using var workbook = new XLWorkbook(stream);

    var sheet = workbook.Worksheets.First();
    var range = sheet.RangeUsed() ?? throw new InvalidOperationException("File Excel không có dữ liệu.");

    var headers = new List<(int Column, string Name)>();
    foreach (var cell in range.FirstRow().Cells())
    {
        var name = cell.GetFormattedString().Trim();
        if (string.IsNullOrWhiteSpace(name)) continue;
        headers.Add((cell.Address.ColumnNumber, name));
    }

    var headerRowNumber = range.FirstRow().RowNumber();
    var rows = new List<object>();
    foreach (var row in range.RowsUsed().Where(r => r.RowNumber() > headerRowNumber))
    {
        var values = new SortedDictionary<int, string>();
        var meta = new SortedDictionary<int, object>();
        foreach (var header in headers)
        {
            var cell = sheet.Cell(row.RowNumber(), header.Column);
            values[header.Column] = ReadCell(cell);
            meta[header.Column] = new
            {
                type = cell.DataType.ToString(),
                numFmtId = cell.Style.NumberFormat.NumberFormatId,
                format = cell.Style.NumberFormat.Format,
            };
        }
        if (values.Values.All(string.IsNullOrWhiteSpace)) continue;

        rows.Add(new { rowNumber = row.RowNumber(), values, meta });
    }

    return new
    {
        file = Path.GetFileName(path),
        sheet = sheet.Name,
        headers = headers.Select(h => new { columnNumber = h.Column, name = h.Name }),
        rows,
    };
}

static string ReadCell(IXLCell cell)
{
    return cell.DataType == XLDataType.DateTime
        ? cell.GetDateTime().ToString("dd/MM/yyyy", CultureInfo.InvariantCulture)
        : cell.GetFormattedString().Trim();
}
