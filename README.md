# fairy - fast approximate metagenomic contig coverage calculation for binning

**Fairy** is a program that can get _approximate_ coverages for metagenomic reads against assembled contigs. Fairy is a derivative of the profiler [sylph](https://github.com/bluenote-1577/sylph) but is modified specifically for **metagenomic binning purposes**.

### Introduction

After metagenomic assembly, optimal workflows require aligning reads for **all metagenomic read samples** against contigs to obtain coverages before using a binner like [metabat2](https://bitbucket.org/berkeleylab/metabat). Unfortunately, all-to-all alignment of samples to assemblies is very slow.

**Fairy** resolves this bottleneck by using a fast k-mer alignment-free method to obtain coverage instead of aligning reads. Fairy's coverages are correlated with aligners (but still approximate). 

Preliminary binning results show that using fairy instead of [BWA](https://github.com/lh3/bwa) for *multi-sample* binning recovers a similar amount of high-quality bins. However, **sylph is 10-1000x faster than BWA for all-to-all coverage calculation**. For single-sample binning, fairy may be slightly worse than BWA, but is still usable.  


##  Install (current version v0.5.1)

#### Option 1: conda install 

FORTHCOMING

#### Option 2: Build from source

Requirements:
1. [rust](https://www.rust-lang.org/tools/install) (version > 1.63) programming language and associated tools such as cargo are required and assumed to be in PATH.
2. A c compiler (e.g. GCC)
3. make
4. cmake

Building takes a few minutes (depending on # of cores).

```sh
git clone https://github.com/bluenote-1577/fairy
cd fairy

# If default rust install directory is ~/.cargo
cargo install --path . 
fairy coverage test_files/*
```
#### Option 3: Pre-built x86-64 linux statically compiled executable

If you're on an x86-64 system, you can download the binary and use it without any installation. 

```sh
wget https://github.com/bluenote-1577/fairy/releases/download/latest/fairy
chmod +x fairy
./fairy -h
```

Note: the binary is compiled with a different set of libraries (musl instead of glibc), probably impacting performance. 

## Quick start

```sh
# sketch/index short reads
fairy sketch -1 *_1.fastq.gz -2 *_2.fastq.gz -d sketch_dir

# sketch/index long reads
fairy sketch -r long_reads.fq -d sketch_dir

# calculate coverage
fairy coverage sketch_dir/*.bcsp contigs.fa -t 10 -o coverage.tsv
```

## Output

The output is compatible with the `jgi_summarize_bam_contig_depths` script from metabat2. TODO

## Citing sylph

TODO
