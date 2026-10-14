// Java profile: inherited structural policy and cathedral ornament.
import java.util.List;
public class Profile {
    static String quoted(String value) {
        return "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
    }
    public static void main(String[] args) {
        String name = "Java";
        String arch = "M -.028 -.94 L -.028 -.965 Q 0 -1.02 .028 -.965 L .028 -.94 M 0 -.99 L 0 -.94";
        List<String> extensions = List.of("java");
        String suffixes = String.join(",", extensions.stream().map(Profile::quoted).toList());
        System.out.println("{\"id\":\"java\",\"base\":\"c\",\"name\":" + quoted(name)
          + ",\"extensions\":[" + suffixes + "],\"call_group\":\"java\","
          + "\"color\":\"#b07219\",\"sigil\":\"♜\",\"repeats\":4,\"collar\":[{\"op\":\"path\",\"data\":" + quoted(arch) + "}]}");
    }
}
