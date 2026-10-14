// C# profile: class-family policy and diamond lattice ornament.
using System;
using System.Text;
public static class Profile {
    static string Quoted(string value) {
        return "\"" + value.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";
    }
    public static void Main() {
        Console.OutputEncoding = new UTF8Encoding(false);
        string name = "C#";
        string[] extensions = { "cs" };
        string[] quotedExtensions = Array.ConvertAll(extensions, Quoted);
        Console.WriteLine("{\"id\":\"csharp\",\"base\":\"java\",\"name\":" + Quoted(name)
          + ",\"extensions\":[" + String.Join(",", quotedExtensions) + "],\"call_group\":\"csharp\","
          + "\"color\":\"#7355dd\",\"sigil\":\"✧\",\"repeats\":8,\"collar\":[{\"op\":\"polygon\",\"cx\":0,\"cy\":-0.964,\"r\":0.031,\"n\":4,\"step\":1,\"rotation_degrees\":0}]}");
    }
}
