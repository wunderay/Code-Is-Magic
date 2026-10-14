"""Independent ZIP-reader verification of canonical SVGZ recovery."""
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
    for path,content in data.items():(root/path).write_bytes(content)
    (root/"run.sh").chmod(0o755)
    if os.name!="nt":(root/"link").symlink_to("spell.rs")
    magic=Path(temp)/"source.svgz"
    recovered=Path(temp)/"recovered.zip"
    subprocess.run(["cargo","run","--locked","--",str(root),"--output",str(magic)],check=True,capture_output=True)
    assert magic.read_bytes().startswith(b"\x1f\x8b")
    first=magic.read_bytes()
    subprocess.run(["cargo","run","--locked","--",str(root),"--output",str(magic)],check=True,capture_output=True)
    assert magic.read_bytes()==first, "Canonical transmutation must be byte deterministic"
    subprocess.run(["cargo","run","--locked","--","--decode",str(magic),"--output",str(recovered)],check=True,capture_output=True)
    positional=Path(temp)/"positional.zip"
    subprocess.run(["cargo","run","--locked","--",str(magic),"--output",str(positional)],check=True,capture_output=True)
    assert positional.read_bytes()==recovered.read_bytes()
    target=Path(json.loads(subprocess.check_output(["cargo","metadata","--no-deps","--format-version=1"]))["target_directory"])
    binary=target/"debug"/("code-is-magic.exe" if os.name=="nt" else "code-is-magic")
    legacy_png=Path(temp)/"forbidden.png"
    failed=subprocess.run([str(binary),str(root),"--output",str(legacy_png)],capture_output=True)
    assert failed.returncode!=0 and not legacy_png.exists()
    assert b"one canonical format" in failed.stderr.lower()
    ordinary=Path(temp)/"ordinary.png";ordinary.write_bytes(b"not a source-carrying image")
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
        for entry in manifest["entries"]:assert z.read("repository/"+entry["path"])==base64.b64decode(entry["data"])
        assert z.testzip() is None
    print("Canonical CLI: SVGZ only, deterministic output, exact ZIP recovery, invalid/legacy-output rejection PASS")

with tempfile.TemporaryDirectory(prefix="magic-keyed-") as temp:
    root=Path(temp)/"source";root.mkdir()
    (root/"spell.rs").write_bytes(b"\xef\xbb\xbffn spell() {\r\n  shine();\r\n}\r\nfn shine() {}\r\n")
    (root/"binary.dat").write_bytes(bytes(range(256)))
    keyfile=Path(temp)/"secret.key"
    def run(*args,env=None,input=None):return subprocess.run([str(binary),*map(str,args)],capture_output=True,env=env,input=input)
    assert run("--generate-key",keyfile).returncode==0
    key=keyfile.read_text().strip();assert len(key)==64
    if os.name!="nt":assert keyfile.stat().st_mode&0o777==0o600
    assert run("--generate-key",keyfile).returncode!=0
    assert keyfile.read_text().strip()==key
    env={**os.environ,"CODE_IS_MAGIC_KEY":key}
    magic=Path(temp)/"keyed.svgz"
    assert run(root,"--output",magic,env=env).returncode==0
    original_magic=magic.read_bytes()
    recovered=Path(temp)/"recovered.zip"
    assert run(magic,"--output",recovered,"--key-file","-",input=key.encode()).returncode==0
    with zipfile.ZipFile(recovered) as z:
        for file in root.iterdir():assert z.read("repository/"+file.name)==file.read_bytes()
    original_zip=recovered.read_bytes()
    for wrong in [None,"02"*32,"invalid-secret"]:
        badenv={k:v for k,v in os.environ.items() if k!="CODE_IS_MAGIC_KEY"}
        if wrong is not None:badenv["CODE_IS_MAGIC_KEY"]=wrong
        failed=run(magic,"--output",recovered,env=badenv)
        assert failed.returncode!=0 and recovered.read_bytes()==original_zip
        if wrong:assert wrong.encode() not in failed.stderr
    names=run("--decode-names",magic,env=env)
    assert names.returncode==0 and [n["name"] for n in json.loads(names.stdout)]==["spell","shine"]
    repeated=Path(temp)/"repeated.svgz"
    assert run(root,"--output",repeated,"--key-file",keyfile,env={**env,"CODE_IS_MAGIC_KEY":"02"*32}).returncode==0
    assert repeated.read_bytes()==original_magic
    (root/"insecure.key").write_text(key)
    assert run(root,"--key-file",root/"insecure.key","--output",repeated).returncode!=0
    print("Keyed canonical CLI: secure generation, deterministic perceptual SVGZ, exact recovery, wrong-key rejection, no secret echo PASS")
