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

with tempfile.TemporaryDirectory(prefix="magic-keyed-") as temp:
    root=Path(temp)/"source";root.mkdir()
    (root/"spell.rs").write_bytes(b"\xef\xbb\xbffn spell() {\r\n  shine();\r\n}\r\nfn shine() {}\r\n")
    (root/"binary.dat").write_bytes(bytes(range(256)))
    keyfile=Path(temp)/"secret.key"
    def run(*args,env=None,input=None):
        return subprocess.run([str(binary),*map(str,args)],capture_output=True,env=env,input=input)
    assert run("--generate-key",keyfile).returncode==0
    key=keyfile.read_text().strip();assert len(key)==64
    if os.name!="nt":assert keyfile.stat().st_mode&0o777==0o600
    assert run("--generate-key",keyfile).returncode!=0
    assert keyfile.read_text().strip()==key
    env={**os.environ,"CODE_IS_MAGIC_KEY":key}
    png=Path(temp)/"keyed.png";svg=Path(temp)/"keyed.svg";offline=Path(temp)/"keyed.json"
    assert run(root,"--output",png,"--json",offline,env=env).returncode==0
    protected=json.loads(offline.read_text());assert set(protected)=={"version","svg"} and protected["version"]==2
    assert key not in offline.read_text() and "spell.rs" not in offline.read_text()
    assert run(root,"--output",svg,"--key-file",keyfile).returncode==0
    recovered=Path(temp)/"recovered.zip"
    for image in [png,svg]:
        assert run(image,"--output",recovered,"--key-file","-",input=key.encode()).returncode==0
        with zipfile.ZipFile(recovered) as z:
            for file in root.iterdir():assert z.read("repository/"+file.name)==file.read_bytes()
        original=recovered.read_bytes()
        for wrong in [None,"02"*32,"invalid-secret"]:
            badenv={k:v for k,v in os.environ.items() if k!="CODE_IS_MAGIC_KEY"}
            if wrong is not None:badenv["CODE_IS_MAGIC_KEY"]=wrong
            failed=run(image,"--output",recovered,env=badenv)
            assert failed.returncode!=0 and recovered.read_bytes()==original
            if wrong:assert wrong.encode() not in failed.stderr
    names=run("--decode-names",svg,env=env)
    assert names.returncode==0 and [n["name"] for n in json.loads(names.stdout)]==["spell","shine"]
    repeated=Path(temp)/"repeated.png"
    assert run(root,"--output",repeated,"--key-file",keyfile,env={**env,"CODE_IS_MAGIC_KEY":"02"*32}).returncode==0
    assert repeated.read_bytes()==png.read_bytes() # explicit file overrides environment
    (root/"insecure.key").write_text(key)
    assert run(root,"--key-file",root/"insecure.key","--output",repeated).returncode!=0
    print("Keyed CLI: secure generation, env/file/stdin, precedence, protected JSON, exact recovery, wrong-key rejection, no secret echo PASS")
