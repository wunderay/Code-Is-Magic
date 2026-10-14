/* Shared brace-language structural rules, authored in C. */
#include <stdio.h>
#include <stddef.h>
static void quoted(const char *value) {
    putchar('"');
    for (const unsigned char *p=(const unsigned char*)value; *p; ++p) {
        if (*p=='"' || *p=='\\') putchar('\\');
        putchar(*p);
    }
    putchar('"');
}
static void strings(const char **items, size_t count) {
    putchar('[');
    for(size_t i=0;i<count;i++){if(i)putchar(',');quoted(items[i]);}
    putchar(']');
}
int main(void) {
    const char *extensions[]={"c","h"};
    const char *controls[]={"if","for","while","switch","catch","match","with","sizeof","return","new","assert","super"};
    const char *prefixes[]={".","::","return","=","=>","new",",","if","while","switch","match","!","&&","||"};
    fputs("{\"id\":\"c\",\"name\":\"C\",\"extensions\":",stdout);strings(extensions,2);
    fputs(",\"call_group\":\"c-family\",\"color\":\"#555555\",\"line_comment\":\"//\",\"block_comments\":true,\"raw_strings\":false,\"lifetimes\":false,\"triple_quotes\":false,\"indented_declarations\":{},",stdout);
    /* Retain the existing broad structural heuristic; these are not compiler grammars. */
    fputs("\"braced_declarations\":{\"struct\":\"struct\",\"union\":\"struct\",\"enum\":\"enum\",\"class\":\"class\",\"record\":\"class\",\"interface\":\"interface\",\"trait\":\"interface\",\"impl\":\"impl\"},",stdout);
    fputs("\"type_keyword\":\"type\",\"type_kinds\":{\"struct\":\"struct\",\"interface\":\"interface\"},\"type_default_kind\":\"struct\",\"object_keywords\":[\"const\",\"let\",\"var\"],\"receiver_keyword\":\"func\",\"required_function_keyword\":\"\",\"arrow_token\":\"=>\",\"controls\":",stdout);
    strings(controls,sizeof(controls)/sizeof(controls[0]));
    fputs(",\"reject_function_prefixes\":",stdout);strings(prefixes,sizeof(prefixes)/sizeof(prefixes[0]));
    fputs(",\"member_operators\":[\".\",\"::\"],\"self_qualifiers\":[\"self\",\"this\",\"Self\"],\"merge_containers\":[\"impl\"],\"sigil\":\"⚙\",\"repeats\":12,\"collar\":[{\"op\":\"path\",\"data\":\"M -.03 -.94 L -.03 -.983 L .03 -.983 L .03 -.94\"}]}\n",stdout);
    return 0;
}
