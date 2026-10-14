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
    # The same positional input accepts a repository/directory or an image.
    positional=Path(temp)/"positional.zip"
    subprocess.run(["cargo","run","--locked","--",str(png),"--output",str(positional)],check=True,capture_output=True)
    assert positional.read_bytes()==recovered.read_bytes()
    svg=Path(temp)/"source.svg"
    subprocess.run(["cargo","run","--locked","--",str(root),"--output",str(svg)],check=True,capture_output=True)
    target=Path(json.loads(subprocess.check_output(["cargo","metadata","--no-deps","--format-version=1"]))["target_directory"])
    binary=target/"debug"/("code-is-magic.exe" if os.name=="nt" else "code-is-magic")
    subprocess.run([str(binary),str(svg)],cwd=temp,check=True,capture_output=True)
    assert (Path(temp)/"recovered.zip").read_bytes()==recovered.read_bytes()
    ordinary=Path(temp)/"ordinary.png"
    ordinary.write_bytes(b"not a source-carrying image")
    invalid_output=Path(temp)/"must-not-exist.zip"
    failed=subprocess.run([str(binary),str(ordinary),"--output",str(invalid_output)],capture_output=True)
    assert failed.returncode!=0 and not invalid_output.exists()
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
    print("Unified CLI input: PNG/SVG → ZIP, default output, invalid-image rejection; exact bytes, UTF-8 paths, CRLF/BOM, executable modes, symlink targets PASS")
