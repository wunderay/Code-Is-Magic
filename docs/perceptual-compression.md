# Perceptual JPEG compression

Code Is Magic supports an opt-in visually imperceptible JPEG carrier for cases where a substantially smaller raster matters more than pixel-for-pixel image identity. The visible pixels are lossy by design; the selected source tree is not.

## Guarantees

- Source recovery remains byte-exact. The JPEG pixel stream is never used to reconstruct source bytes.
- The source snapshot is canonicalized, compressed independently, integrity-checked, and stored in deterministic JPEG APP15 segments.
- With a magic key, the source payload is authenticated and encrypted with the existing AES-256-SIV machinery under a dedicated `perceptual-source/v1` domain before it enters the JPEG container.
- The default visual profile is quality 96, progressive JPEG, RGB 4:4:4 (no chroma subsampling), and optimized Huffman tables.
- The CLI accepts perceptual qualities 95 through 100 only. Lower values are deliberately rejected rather than silently permitting visibly degraded output.
- Given the same canonical snapshot, artwork, key, quality, and pinned codec version, the encoder emits the same bytes.
- Exact PNG, SVG, and SVGZ export behavior is unchanged.

## Container layout

The JPEG is an ordinary standards-compliant JPEG image plus one or more APP15 segments. Each Code Is Magic segment contains:

- magic `CIMJPG01`;
- zero-based chunk sequence and total chunk count;
- total source-payload length;
- CRC-32 of the complete protected payload;
- one chunk of the protected payload.

The source payload itself is the smaller of maximum DEFLATE and Brotli quality 9 with a 4 MiB window. Keyed payloads are wrapped with `CIMVKEY1` after deterministic authenticated encryption. Decoding rejects missing, duplicated, inconsistent, oversized, or CRC-invalid chunks rather than returning partial source.

This separation is intentional. Exact PNGs hide source bits inside RGB pixels, which makes those low bits look like compression-resistant noise. Perceptual JPEG stores the visible image and exact source as different streams, so the visual codec can optimize the artwork without ever being responsible for source recovery.

## CLI

Create a perceptual JPEG with the conservative default profile:

```sh
code-is-magic ./my-project --output magic.jpg
```

Choose a stricter quality inside the guarded range:

```sh
code-is-magic ./my-project --perceptual-quality 98 --output magic.jpg
```

Keyed export and exact source recovery work in both directions:

```sh
code-is-magic ./my-project --key-file ../my-magic.key --output magic.jpg
code-is-magic magic.jpg --key-file ../my-magic.key --output recovered.zip
```

The website exposes the same path through **JPEG ↓** and can recover source from an original Code Is Magic JPEG. A perceptual JPEG does not retain the exact vector drawing, so after source recovery the interactive website deterministically re-renders the recovered source instead of pretending that the lossy JPEG pixels were an exact SVG.

## What “visually imperceptible” means here

This format does not claim pixel equality. It deliberately trades tiny raster differences for a much smaller file while using settings chosen to preserve the project’s difficult content: thin runes, sharp circle edges, braided connections, flat fills, and glow gradients. In representative development measurements, quality 96 at 4:4:4 reduced the visual stream substantially relative to the exact PNG while keeping average channel error around one 8-bit intensity level. Exact measurements depend on the diagram.

Use PNG when decoded RGBA pixels must be identical. Use SVG/SVGZ when exact vector artwork matters. Use perceptual JPEG when the human-visible result and exact source recovery matter but individual raster samples do not.

## Preservation warning

Keep the original Code Is Magic JPEG if you need source recovery. Image editors, social networks, optimization services, or metadata scrubbers may preserve the visible JPEG pixels while deleting APP15 segments. A stripped file remains viewable but no longer contains the exact source capsule. Re-encoding also changes the visible pixels.

This is different from the exact PNG carrier: converting an exact PNG to an ordinary JPEG destroys the PNG’s pixel-embedded source bits. Generate a perceptual JPEG directly from Code Is Magic instead.
