"""Independent ZIP-reader verification of byte-exact CLI image recovery."""
import base64
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile

with tempfile.TemporaryDirectory(prefix="magic-recovery-") as temp:
    root=Path(temp)/"source"
    root.mkdir()
    data={"spell.rs":b"\xef\xbb\xbffn spell() {\r\n  return;\r\n}\r\n", "binary.dat":bytes(range(256)),"run.sh":b"#!/bin/sh\n", "配置.txt":"魔法\n".encode()}
    for path,content in data.items():
        (root/path).write_bytes(content)
    (root/"run.sh").chmod(0o755)
    if os.name!="nt":(root/"link").symlink_to("spell.rs")
    png=Path(temp)/"source.png"
    recovered=Path(temp)/"recovered.zip"
    subprocess.run(["cargo","run","--locked","--",str(root),"--output",str(png)],check=True,capture_output=True)
    subprocess.run(["cargo","run","--locked","--","--decode",str(png),"--output",str(recovered)],check=True,capture_output=True)
    with zipfile.ZipFile(recovered) as z:
        for path,content in data.items():assert z.read("repository/"+path)==content,path
        assert (z.getinfo("repository/run.sh").external_attr>>16)&0o111==0o111
        if os.name!="nt":
            assert z.read("repository/link")==b"spell.rs"
            assert (z.getinfo("repository/link").external_attr>>16)&0o170000==0o120000
        manifest=json.loads(z.read("restoration.json"))
        for entry in manifest["entries"]:
            assert z.read("repository/"+entry["path"])==base64.b64decode(entry["data"])
        assert z.testzip() is None
    print("Independent CLI PNG → ZIP recovery: exact bytes, UTF-8 paths, CRLF/BOM, executable modes, symlink targets PASS")
