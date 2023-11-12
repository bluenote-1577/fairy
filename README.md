# sylph -  ANI genome querying and metagenomic profiling for shotgun metagenomes 

## Introduction

**sylph** is a program that can perform ultrafast (1) **ANI querying** or (2) **metagenomic profiling** for metagenomic shotgun samples. 

**ANI querying**: sylph can search a genome, e.g. E. coli, against your sample. If sylph gives an estimate of 97% ANI, then a genome is contained in your sample with 97% ANI to the queried E. coli genome. 

**Metagenomic profiling**: Like e.g. Kraken or MetaPhlAn, sylph can determine what species are in your sample and their abundances, as well as their _ANI to the database_.

### Why sylph?

1. **Accurate ANIs down to 0.1x coverage**: for bacterial ANI queries of > 90% ANI, sylph can give accurate ANI estimates down to 0.1x coverage and often even lower.

2. **Precise profiling**: Our tests show that sylph is more precise than Kraken, about as precise and sensitive as marker gene methods (MetaPhlAn, mOTUs) but with possibly better abundance estimates. 

3. **Ultrafast, multithreaded, multi-sample**: sylph is > 100x faster than MetaPhlAn for multi-sample processing. sylph only takes 10GB of RAM for profiling against the entire GTDB-R214 database (85k genomes).

4. **Easily customized databases**: sylph does not require taxonomic information, so anything you can profile against metagenome-assembled genomes (MAGs), viruses, eukaryotes, even assembled contigs, etc. Taxonomic information can be incorporated downstream for traditional profiling reports. 

### How does sylph work?

sylph uses a k-mer containment method, similar to sourmash or Mash. sylph's novelty lies in **using a statistical technique to correct ANI for low coverage genomes** within the sample, allowing accurate ANI queries for even low abundance genomes.

## WARNING EARLY DEVELOPMENT

sylph is being developed rapidly. It has not been officially released yet. I am planning on releasing sylph officially in the next 1-3 months (October-December 2023).  

The following may change:
   - any sketches you use may not work by the next release
   - the command line options
   - Parameters will change 

##  Install (current version v0.4.0)

#### Option 1: conda install 
[![Anaconda-Server Badge](https://anaconda.org/bioconda/sylph/badges/version.svg)](https://anaconda.org/bioconda/sylph)
[![Anaconda-Server Badge](https://anaconda.org/bioconda/sylph/badges/latest_release_date.svg)](https://anaconda.org/bioconda/sylph)

```sh
conda install -c bioconda sylph
```

#### Option 2: Build from source

Requirements:
1. [rust](https://www.rust-lang.org/tools/install) (version > 1.63) programming language and associated tools such as cargo are required and assumed to be in PATH.
2. A c compiler (e.g. GCC)
3. make
4. cmake

Building takes a few minutes (depending on # of cores).

```sh
git clone https://github.com/bluenote-1577/sylph
cd sylph

# If default rust install directory is ~/.cargo
cargo install --path . --root ~/.cargo
sylph query test_files/*
```
#### Option 3: Pre-built x86-64 linux statically compiled executable

If you're on an x86-64 system, you can download the binary and use it without any installation. 

```sh
wget https://github.com/bluenote-1577/sylph/releases/download/latest/sylph
chmod +x sylph
./sylph -h
```

Note: the binary is compiled with a different set of libraries (musl instead of glibc), possibly impacting performance. 

## Quick start

```sh
# all fasta -> one *.syldb; fasta are assumed to be genomes
sylph sketch genome1.fa genome2.fa -o database
#EQUIVALENT: sylph sketch -g genome1.fa genome2.fa -o database

# multi-sample sketching of paired reads
sylph sketch -1 A_1.fq B_1.fq -2 A_2.fq B_2.fq -d output_read_sketch_folder

# multi-sample sketching for single end reads, fastq are assumed to be reads
sylph sketch reads.fq 
#EQUIVALENT: sylph sketch -r reads.fq

# ANI querying 
sylph query database.syldb *.sylsp -t (threads) > ani_queries.tsv

# taxonomic profiling 
sylph profile database.syldb *.sylsp -t (threads) > profiling.tsv
```

See [Pre-sketched databases](#pre-databases) below to download pre-indexed databases. 

## Tutorials and manuals

### [Cookbook](https://github.com/bluenote-1577/sylph/wiki/sylph-cookbook)

For common use-cases and fast explanations, see the above [cookbook](https://github.com/bluenote-1577/sylph/wiki/sylph-cookbook). 

### Tutorials

1. #### [Introduction: 5-minute sylph tutorial outlining basic usage](https://github.com/bluenote-1577/sylph/wiki/5%E2%80%90minute-sylph-tutorial)

### Manuals
1. #### [Sylph's TSV output format](https://github.com/bluenote-1577/sylph/wiki/Output-format)
1. #### [Incoporating taxonomy to get CAMI-like or MetaPhlAn-like outputs for GTDB (and custom taxonomy)](https://github.com/bluenote-1577/sylph/wiki/MetaPhlAn-or-CAMI%E2%80%90like-output-with-the-GTDB-database)

<a name="pre-databases"></a>
## Pre-sketched databases

We have some pre-sketched databases available for download below. 

### Pre-sketched GTDB r214 database (85,202 genomes). Works with v0.3.0 - current

1. `-c 200`, more sensitive database (10 GB): https://storage.googleapis.com/sylph-stuff/v0.3-c200-gtdb-r214.syldb
2. `-c 1000` more efficient, less sensitive database (2 GB): https://storage.googleapis.com/sylph-stuff/v0.3-c1000-gtdb-r214.syldb

### Pre-sketched IMG/VR4 database for high-confidence vOTU representatives (2,917,516 viral genomes). Works with v0.3.0 - current
1. `-c 200` (2GB): https://storage.googleapis.com/sylph-stuff/imgvr_c200_v0.3.0.syldb

Quick usage example

```sh
# faster, less sensitive database
wget https://storage.googleapis.com/sylph-stuff/v0.3-c1000-gtdb-r214.syldb
sylph profile reads.fq v0.3-c200-gtdb-r214.syldb -t 30 > results.tsv
```

## Citing sylph

Jim Shaw and Yun William Yu. Ultrafast, coverage-corrected genome similarity queries for metagenomic shotgun samples with sylph (Preprint to be released soon). 

