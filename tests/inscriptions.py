"""Independent geometric checks of the canonical perceptual inscription cells."""
import gzip
import json
import math
import os
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parent.parent
ATLAS = json.loads((ROOT / 'src/glyphs.json').read_text())
assert len(ATLAS) >= 450
families = {glyph['family'] for glyph in ATLAS.values()}
assert len(families) == 10
assert all(g['width'] > 0 and g['height'] > 0 and g['path'] for g in ATLAS.values())
subprocess.run(['cargo', 'build', '--locked'], cwd=ROOT, check=True, capture_output=True)
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version=1'], cwd=ROOT))
binary = Path(metadata['target_directory']) / 'debug' / ('code-is-magic.exe' if os.name == 'nt' else 'code-is-magic')
namespace = {'s': 'http://www.w3.org/2000/svg'}
with tempfile.TemporaryDirectory(prefix='magic-inscriptions-') as temp:
    temp = Path(temp)
    source = temp / 'source'
    source.mkdir()
    content = ''.join(f'fn spell_{i}() {{\n' + ''.join(f'    let n{j} = {j};\n' for j in range([1, 4, 25, 64, 400][i % 5])) + '}\n' for i in range(100))
    (source / 'spells.rs').write_text(content)
    for key in ['', '01' * 32]:
        env = {k: v for k, v in os.environ.items() if k != 'CODE_IS_MAGIC_KEY'}
        if key:env['CODE_IS_MAGIC_KEY'] = key
        output = temp / 'out.svgz'
        command = [str(binary), str(source), '--output', str(output), '--language-colors']
        subprocess.run(command, env=env, check=True, capture_output=True)
        encoded = output.read_bytes()
        subprocess.run(command, env=env, check=True, capture_output=True)
        assert output.read_bytes() == encoded
        svg = gzip.decompress(encoded).decode()
        document = ET.fromstring(svg)
        assert document.get('data-inscription-version') == '2'
        assert not document.findall('.//s:text', namespace)
        bands = document.findall('.//s:g[@data-inscription-band]', namespace)
        seen = set();styles = set();fills = []
        for band in bands:
            inner, outer, middle = [float(band.get('data-' + k)) for k in ['inner', 'outer', 'middle']]
            styles.add(band.get('data-style'))
            cells = band.findall('s:g[@data-cell]', namespace)
            assert len(cells) >= 12
            assert abs(sum(float(c.get('data-sector')) for c in cells) - 2 * math.pi) < 1e-7
            for cell in cells:
                sector = float(cell.get('data-sector'))
                glyphs = cell.findall('s:use[@data-glyph]', namespace)
                assert glyphs, 'Missing shared glyph outlines'
                for glyph in glyphs:
                    seen.add(glyph.get('data-family'))
                    width, height, offset = [float(glyph.get('data-' + k)) for k in ['width', 'height', 'offset']]
                    near = middle - offset - height / 2
                    far = math.hypot(middle - offset + height / 2, width / 2)
                    assert near >= inner - 0.00001, (near, inner)
                    assert far <= outer + 0.00001, (far, outer)
                    assert math.atan2(width / 2, near) < sector / 2, 'Glyph crosses its angular cell'
                if len(glyphs) == 1:fills.append(float(glyphs[0].get('data-height')) / (outer - inner))
                else:
                    top, bottom = glyphs
                    assert float(top.get('data-offset')) + float(top.get('data-height')) / 2 < float(bottom.get('data-offset')) - float(bottom.get('data-height')) / 2
        assert styles == {'0', '1', '2', '3'}
        assert seen == families
        assert sum(fills) / len(fills) > 0.65, 'Glyphs underfill the available radial height'
        names = json.loads(subprocess.check_output([str(binary), '--decode-names', str(output)], env=env))
        assert len(names) == 100
        recovered = temp / 'recovered.zip'
        subprocess.run([str(binary), str(output), '--output', str(recovered)], env=env, check=True, capture_output=True)
        with zipfile.ZipFile(recovered) as archive:
            assert archive.read('repository/spells.rs') == (source / 'spells.rs').read_bytes()
print('Canonical inscriptions PASS: 10 symbol families, geometric spacing, deterministic perceptual SVGZ, exact name/source recovery')
