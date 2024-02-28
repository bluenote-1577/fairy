# fairy - fast approximate contig coverage for metagenomic binning

## Introduction - multi-sample coverage problem

After metagenomic assembly, optimal workflows require aligning **all metagenomic reads** against all assemblies to obtain coverages. Then, metagenome-assembled genomes (MAGs) are generated using a binner like [metabat2](https://bitbucket.org/berkeleylab/metabat). 

Unfortunately, all-to-all alignment of samples to assemblies **is very slow**.

**Fairy** resolves this bottleneck by using a fast k-mer alignment-free method to obtain coverage instead of aligning reads. Fairy's coverages are correlated with aligners (but still approximate). However, **fairy is 10-1000x faster than BWA for all-to-all coverage calculation**. 

### Important: fairy is designed for **multi-sample** usage and short reads or nanopore reads. Do not use fairy for **single-sample** binning. 

### Short-reads 
Fairy seems to be comparable to [BWA](https://github.com/lh3/bwa) for **multi-sample** binning (maybe a +5% to -15% loss in sensitivity). Preliminary testing indicates that fairy may perform as good as (and sometimes better than) BWA on host-associated datasets and slightly worse (but usable) on environmental datasets.

### Long-reads
**Non-HiFi:** For simplex nanopore reads, fairy seems to be comparable with minimap2. 

**HiFi (strain-resolved assemblies)**: Fairy is worse than minimap2 for strain-resolved assemblies when using >99.9% identity reads (using e.g. hifiasm or meta-mdbg). 

##  Install (current version v0.5.3)

#### Option 1: conda install 

```sh
mamba install -c bioconda fairy
# conda install -c bioconda fairy
```

**Warning**: If you're using linux, conda may require AVX2 instructions (e.g. a newer CPU). Source install (option 2) and the static binary (option 3) should still work. 

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
fairy -h 
```
#### Option 3: Pre-built x86-64 linux statically compiled executable

If you're on an x86-64 Linux system, you can download the binary and use it without any installation. 

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

# rename the sketches if filenames are identical
fairy sketch -r dir1/reads.fq dir2/reads.fq -S sample1 sample2 -d sketch_dir

# calculate coverage
fairy coverage sketch_dir/*.bcsp contigs.fa -t 10 -o coverage.tsv
```

## Output

### MetaBAT2 format (default)

The default output is compatible with the `jgi_summarize_bam_contig_depths` script from MetaBAT2 (the column names are different, however). 

```sh
contigName  contigLen  totalAvgDepth  reads1.fq  reads1.fq-var  reads2.fq  reads2.fq-var  ...
contig_1    38370      1.4            1.4        1.1100          0       0
...
```

1. First three columns give the name, the length, and average coverage.
2. The next columns are `mean coverage` and `coverage variance` for each sample.

The above output can be fed directly into MetaBAT2 with default parameters. 

### MaxBin2 format

Alternatively, `--maxbin-format` works directly with MaxBin2 and is also available. This removes the variance columns as well as the `contigLen` and `totalAvgDepth` columns. 

## Citing fairy

Forthcoming.
