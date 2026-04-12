# Codebook Brain implementieren

bgz17/PCDVQ komprimierte Weights für lokale Inference auf Pi 4.

1. Codebook-Loader: mmap .bgz Dateien aus /opt/pi-audio/codebook/
2. Lookup-Kernel: Skalar zuerst, dann NEON optimieren
3. Token-Generator: Codebook → Logits → Sampling → Text
4. Benchmark: tok/s auf Pi 4 vs. x86
5. Integration in wol_brain Cascade (Stufe 0)

Repos: AdaWorldAPI/ndarray (bgz17), AdaWorldAPI/lance-graph
Feature-Flags: scalar, neon, avx2, avx512, amx

Schlüsselerkenntnis: Das ist KEIN Matmul. Es ist Lookup. O(1) pro Token.
