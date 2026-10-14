// C++ dialect pairing and linked-square clockwork.
#include <iostream>
#include <string>
static std::string quote(const std::string &s) {
    std::string result="\"";
    for(char c:s){if(c=='"'||c=='\\')result+='\\';result+=c;}
    return result+'"';
}
static void square(double x) {
    std::cout << "{\"op\":\"polygon\",\"cx\":" << x
      << ",\"cy\":-0.964,\"r\":0.027,\"n\":4,\"step\":1,\"rotation_degrees\":45}";
}
int main() {
    const std::string name="C++";
    std::cout << "{\"id\":\"cpp\",\"base\":\"c\",\"name\":" << quote(name)
      << ",\"extensions\":[\"cpp\",\"hpp\",\"cc\"],\"call_group\":\"c-family\","
      << "\"sigil\":\"⚙\",\"repeats\":16,\"collar\":[";
    square(-0.017);std::cout << ',';square(0.017);std::cout << "]}\n";
}
