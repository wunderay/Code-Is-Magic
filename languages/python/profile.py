"""Python indentation, comments, strings, declarations and lunar design."""
import json
profile = {
    "id": "python", "base": "c", "name": "Python", "extensions": ["py"],
    "call_group": "python", "line_comment": "#", "block_comments": False,
    "triple_quotes": True,
    "indented_declarations": {"def": "function", "class": "class"},
    "sigil": "☽", "repeats": 6,
    "collar": [{"op": "path", "data": "M -.035 -.965 Q 0 -.925 .035 -.965 Q 0 -.995 -.035 -.965 Z"}],
}
print(json.dumps(profile, ensure_ascii=False))
