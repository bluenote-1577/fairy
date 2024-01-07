use crate::cmdline::*;
use scalable_cuckoo_filter::ScalableCuckooFilter;
use scalable_cuckoo_filter::ScalableCuckooFilterBuilder;

use fxhash::FxHashMap;
use fxhash::FxHashSet;
use fxhash::FxHasher;
use memory_stats::memory_stats;
use std::fs;
use std::thread;
use std::time::Duration;

use crate::constants::*;
use crate::seeding::*;
use crate::types::*;
use log::*;
use needletail::parse_fastx_file;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::io::{prelude::*, BufReader};
use std::path::Path;
type Marker = u32;

pub fn check_vram_and_block(max_ram: usize, file: &str) {
    if let Some(usage) = memory_stats() {
        let mut gb_usage_curr = usage.virtual_mem as f64 / 1_000_000_000 as f64;
        if (max_ram as f64) < gb_usage_curr {
            log::debug!(
                "Max memory reached. Blocking sketch for {}. Curr memory {}, max mem {}",
                file,
                gb_usage_curr,
                max_ram
            );
        }
        while (max_ram as f64) < gb_usage_curr {
            let five_second = Duration::from_secs(1);
            thread::sleep(five_second);
            if let Some(usage) = memory_stats() {
                gb_usage_curr = usage.virtual_mem as f64 / 1_000_000_000 as f64;
                if (max_ram as f64) >= gb_usage_curr {
                    log::debug!("Sketching for {} freed", file);
                }
            } else {
                break;
            }
        }
    }
}

pub fn extract_markers(string: &[u8], kmer_vec: &mut Vec<u64>, c: usize, k: usize) {
    #[cfg(any(target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            use crate::avx2_seeding::*;
            unsafe {
                extract_markers_avx2(string, kmer_vec, c, k);
            }
        } else {
            fmh_seeds(string, kmer_vec, c, k);
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        fmh_seeds(string, kmer_vec, c, k);
    }
}

pub fn extract_markers_positions(
    string: &[u8],
    kmer_vec: &mut Vec<(usize, usize, u64)>,
    c: usize,
    k: usize,
    contig_number: usize,
) {
    #[cfg(any(target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            use crate::avx2_seeding::*;
            unsafe {
                extract_markers_avx2_positions(string, kmer_vec, c, k, contig_number);
            }
        } else {
            fmh_seeds_positions(string, kmer_vec, c, k, contig_number);
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        fmh_seeds_positions(string, kmer_vec, c, k, contig_number);
    }
}

pub fn is_fastq(file: &str) -> bool {
    if file.ends_with(".fq")
        || file.ends_with(".fnq")
        || file.ends_with(".fastq")
        || file.ends_with(".fq.gz")
        || file.ends_with(".fnq.gz")
        || file.ends_with(".fastq.gz")
    {
        return true;
    } else {
        return false;
    }
}

pub fn is_fasta(file: &str) -> bool {
    if file.ends_with(".fa")
        || file.ends_with(".fna")
        || file.ends_with(".fasta")
        || file.ends_with(".fa.gz")
        || file.ends_with(".fna.gz")
        || file.ends_with(".fasta.gz")
    {
        return true;
    } else {
        return false;
    }
}

fn check_args_valid(args: &SketchArgs) {
    let level;
    if args.trace {
        level = log::LevelFilter::Trace;
    } else if args.debug {
        level = log::LevelFilter::Debug;
    } else {
        level = log::LevelFilter::Info;
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    simple_logger::SimpleLogger::new()
        .with_level(level)
        .init()
        .unwrap();

    if args.first_pair.is_empty()
        && args.second_pair.is_empty()
        && args.reads.is_none()
        && args.list_reads.is_none()
        && args.list_first_pair.is_none()
        && args.list_second_pair.is_none()
    {
        error!("No input sequences found; see sylph sketch -h for help. Exiting.");
        std::process::exit(1);
    }

    if args.fpr < 0. || args.fpr >= 1. {
        error!("Invalid FPR for sketching. Must be in [0,1).");
        std::process::exit(1);
    }
}

fn parse_reads(
    args: &SketchArgs,
    read_inputs: &mut Vec<String>,
) {
    if let Some(reads_syl_in) = args.reads.clone() {
        for rd_file in reads_syl_in {
            read_inputs.push(rd_file);
        }
    }

    if args.list_reads.is_some() {
        let file_reads = args.list_reads.as_ref().unwrap();
        parse_line_file(file_reads, read_inputs);
    }
}

fn parse_paired_end_reads(
    args: &SketchArgs,
    first_pairs: &mut Vec<String>,
    second_pairs: &mut Vec<String>,
) {
    if args.first_pair.len() != args.second_pair.len() {
        error!("Different number of paired sequences. Exiting.");
        std::process::exit(1);
    }

    for f in args.first_pair.iter() {
        first_pairs.push(f.clone());
    }

    for f in args.second_pair.iter() {
        second_pairs.push(f.clone());
    }

    if args.list_first_pair.is_some() {
        let file_first_pair = args.list_first_pair.as_ref().unwrap();
        parse_line_file(file_first_pair, first_pairs);
    }

    if args.list_second_pair.is_some() {
        let file_second_pair = args.list_second_pair.as_ref().unwrap();
        parse_line_file(file_second_pair, second_pairs)
    }

    if first_pairs.len() != second_pairs.len() {
        error!("Different number of paired sequences. Exiting.");
        std::process::exit(1);
    }
}

fn parse_line_file(file_name: &str, vec: &mut Vec<String>) {
    let file = File::open(file_name).unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        vec.push(line.unwrap());
    }
}

fn parse_sample_names(args: &SketchArgs) -> Option<Vec<String>> {
    if args.list_sample_names.is_none() && args.sample_names.is_none() {
        return None;
    } else {
        let mut sample_names = vec![];
        if let Some(file) = &args.list_sample_names {
            parse_line_file(file, &mut sample_names);
            return Some(sample_names);
        }
        if let Some(vec) = &args.sample_names {
            sample_names.extend(vec.clone());
        }
        return Some(sample_names);
    }
}

pub fn sketch(args: SketchArgs) {
    let mut read_inputs = vec![];
    let mut first_pairs = vec![];
    let mut second_pairs = vec![];

    check_args_valid(&args);
    parse_reads(&args, &mut read_inputs);
    parse_paired_end_reads(&args, &mut first_pairs, &mut second_pairs);

    let sample_names = parse_sample_names(&args);
    if let Some(names) = &sample_names {
        if names.len() != first_pairs.len() + read_inputs.len() {
            log::error!("Sample name length is not equal to the number of reads. Exiting");
            std::process::exit(1);
        }
    }

    let mut max_ram = usize::MAX;
    if args.max_ram.is_some() {
        max_ram = args.max_ram.unwrap();
        if max_ram < 7 {
            log::error!("Max ram must be >= 7. Exiting.");
            std::process::exit(1);
        }
    }

    if !first_pairs.is_empty() && !second_pairs.is_empty() {
        info!("Sketching paired sequences...");
        let iter_vec: Vec<usize> = (0..first_pairs.len()).into_iter().collect();
        iter_vec.into_par_iter().for_each(|i| {
            let read_file1 = &first_pairs[i];
            let read_file2 = &second_pairs[i];
            check_vram_and_block(max_ram, read_file1);

            let mut sample_name = None;
            if let Some(name) = &sample_names {
                sample_name = Some(name[i].clone());
            }
            let read_sketch_opt = sketch_pair_sequences(
                read_file1,
                read_file2,
                args.c,
                args.k,
                sample_name.clone(),
                args.no_dedup,
                args.fpr,
            );
            if read_sketch_opt.is_some() {
                let res = fs::create_dir_all(&args.sample_output_dir);
                if res.is_err() {
                    error!("Could not create directory at {}", args.sample_output_dir);
                    std::process::exit(1);
                }
                let pref = Path::new(&args.sample_output_dir);
                let read_sketch = read_sketch_opt.unwrap();

                let sketch_name;
                if sample_name.is_some() {
                    sketch_name = read_sketch.sample_name.as_ref().unwrap();
                } else {
                    sketch_name = &read_sketch.file_name;
                }

                let read_file_path = Path::new(&sketch_name).file_name().unwrap();
                let file_path = pref.join(&read_file_path);

                let file_path_str = format!(
                    "{}.paired{}",
                    file_path.to_str().unwrap(),
                    SAMPLE_FILE_SUFFIX
                );

                let mut read_sk_file = BufWriter::new(
                    File::create(&file_path_str)
                        .expect(&format!("{} path not valid; exiting ", file_path_str)),
                );

                let enc = SequencesSketchEncode::new(read_sketch);
                bincode::serialize_into(&mut read_sk_file, &enc).unwrap();
                info!("Sketching {} complete.", file_path_str);
            }
        });
    }

    if !read_inputs.is_empty() {
        info!("Sketching non-paired sequences...");
    }

    let iter_vec: Vec<usize> = (0..read_inputs.len()).into_iter().collect();
    iter_vec.into_par_iter().for_each(|i| {
        let pref = Path::new(&args.sample_output_dir);
        std::fs::create_dir_all(pref)
            .expect("Could not create directory for output sample files (-d). Exiting...");

        let read_file = &read_inputs[i];

        check_vram_and_block(max_ram, read_file);
        let mut sample_name = None;
        if let Some(name) = &sample_names {
            sample_name = Some(name[i + first_pairs.len()].clone());
        }

        let read_sketch_opt;
        read_sketch_opt = sketch_sequences_needle(
            read_file,
            args.c,
            args.k,
            sample_name.clone(),
            args.no_dedup,
        );

        if read_sketch_opt.is_some() {
            let read_sketch = read_sketch_opt.unwrap();
            let sketch_name;
            if sample_name.is_some() {
                sketch_name = read_sketch.sample_name.as_ref().unwrap();
            } else {
                sketch_name = &read_sketch.file_name;
            }
            let read_file_path = Path::new(&sketch_name).file_name().unwrap();
            let file_path = pref.join(&read_file_path);

            let file_path_str = format!("{}{}", file_path.to_str().unwrap(), SAMPLE_FILE_SUFFIX);

            let mut read_sk_file = BufWriter::new(
                File::create(&file_path_str)
                    .expect(&format!("{} path not valid; exiting ", file_path_str)),
            );

            let enc = SequencesSketchEncode::new(read_sketch);
            bincode::serialize_into(&mut read_sk_file, &enc).unwrap();
            info!("Sketching {} complete.", file_path_str);
        }
    });

    info!("Finished.");
}

pub fn sketch_genome_individual(
    c: usize,
    k: usize,
    ref_file: &str,
    min_spacing: usize,
    pseudotax: bool,
) -> Vec<GenomeSketch> {
    let reader = parse_fastx_file(&ref_file);
    if !reader.is_ok() {
        warn!("{} is not a valid fasta/fastq file; skipping.", ref_file);
        return vec![];
    } else {
        let mut reader = reader.unwrap();
        let mut return_vec = vec![];
        while let Some(record) = reader.next() {
            let mut return_genome_sketch = GenomeSketch::default();
            return_genome_sketch.c = c;
            return_genome_sketch.k = k;
            return_genome_sketch.file_name = ref_file.to_string();
            if record.is_ok() {
                let mut pseudotax_track_kmers = vec![];
                let mut kmer_vec = vec![];
                let record = record.expect(&format!("Invalid record for file {} ", ref_file));
                let contig_name = String::from_utf8_lossy(record.id()).to_string();
                return_genome_sketch.first_contig_name = contig_name;
                let seq = record.seq();

                extract_markers_positions(&seq, &mut kmer_vec, c, k, 0);
                //fmh_seeds_positions(&seq, &mut kmer_vec, c, k, 0);

                let mut kmer_set = MMHashSet::default();
                let mut duplicate_set = MMHashSet::default();
                let mut new_vec = Vec::with_capacity(kmer_vec.len());
                kmer_vec.sort();

                for (_, _pos, km) in kmer_vec.iter() {
                    if !kmer_set.contains(&km) {
                        kmer_set.insert(km);
                    } else {
                        duplicate_set.insert(km);
                    }
                }

                let mut last_pos = 0;
                for (_, pos, km) in kmer_vec.iter() {
                    if !duplicate_set.contains(&km) || true{
                        if last_pos == 0 || pos - last_pos > min_spacing {
                            new_vec.push(*km);
                            last_pos = *pos;
                        } else if pseudotax {
                            pseudotax_track_kmers.push(*km);
                        }
                    }
                }

                return_genome_sketch.gn_size = record.seq().len();
                return_genome_sketch.genome_kmers = new_vec;
                return_genome_sketch.min_spacing = min_spacing;
                if pseudotax {
                    return_genome_sketch.pseudotax_tracked_nonused_kmers =
                        Some(pseudotax_track_kmers);
                }
                return_vec.push(return_genome_sketch);
            } else {
                warn!("File {} is not a valid fasta/fastq file", ref_file);
                return vec![];
            }
        }
        return return_vec;
    }
}

pub fn sketch_genome(
    c: usize,
    k: usize,
    ref_file: &str,
    min_spacing: usize,
    pseudotax: bool,
) -> Option<GenomeSketch> {
    let reader = parse_fastx_file(&ref_file);
    let mut vec = vec![];
    let mut pseudotax_track_kmers = vec![];
    if !reader.is_ok() {
        warn!("{} is not a valid fasta/fastq file; skipping.", ref_file);
        return None;
    } else {
        let mut reader = reader.unwrap();
        let mut first = true;
        let mut return_genome_sketch = GenomeSketch::default();
        return_genome_sketch.c = c;
        return_genome_sketch.k = k;
        return_genome_sketch.file_name = ref_file.to_string();
        let mut contig_number = 0;
        while let Some(record) = reader.next() {
            if record.is_ok() {
                let record = record.expect(&format!("Invalid record for file {} ", ref_file));
                if first {
                    let contig_name = String::from_utf8_lossy(record.id()).to_string();
                    return_genome_sketch.first_contig_name = contig_name;
                    first = false;
                }
                let seq = record.seq();

                return_genome_sketch.gn_size += seq.len();
                extract_markers_positions(&seq, &mut vec, c, k, contig_number);

                contig_number += 1
            } else {
                warn!("File {} is not a valid fasta/fastq file", ref_file);
                return None;
            }
        }
        let mut kmer_set = MMHashSet::default();
        let mut duplicate_set = MMHashSet::default();
        let mut new_vec = Vec::with_capacity(vec.len());
        vec.sort();
        for (_, _, km) in vec.iter() {
            if !kmer_set.contains(&km) {
                kmer_set.insert(km);
            } else {
                duplicate_set.insert(km);
            }
        }

        let mut last_pos = 0;
        let mut last_contig = 0;
        for (contig, pos, km) in vec.iter() {
            if !duplicate_set.contains(&km) {
                if last_pos == 0 || last_contig != *contig || pos - last_pos > min_spacing {
                    new_vec.push(*km);
                    last_contig = *contig;
                    last_pos = *pos;
                } else if pseudotax {
                    pseudotax_track_kmers.push(*km);
                }
            }
        }
        return_genome_sketch.genome_kmers = new_vec;
        return_genome_sketch.min_spacing = min_spacing;
        if pseudotax {
            return_genome_sketch.pseudotax_tracked_nonused_kmers = Some(pseudotax_track_kmers);
        }
        return Some(return_genome_sketch);
    }
}

#[inline]
fn pair_kmer_single(s1: &[u8]) -> Option<([Marker; 2], [Marker; 2])> {
    let k = std::mem::size_of::<Marker>() * 4;
    if s1.len() < 4 * k + 2 {
        return None;
    } else {
        let mut kmer_f = 0;
        let mut kmer_g = 0;
        let mut kmer_r = 0;
        let mut kmer_t = 0;
        let halfway = s1.len() / 2;
        // len(s1)/2 + (k-1)* 2 + 2 < len(s1)
        for i in 0..k {
            let nuc_1 = BYTE_TO_SEQ[s1[2 * i] as usize] as Marker;
            let nuc_2 = BYTE_TO_SEQ[s1[2 * i + halfway] as usize] as Marker;
            let nuc_3 = BYTE_TO_SEQ[s1[1 + 2 * i] as usize] as Marker;
            let nuc_4 = BYTE_TO_SEQ[s1[1 + 2 * i + halfway] as usize] as Marker;

            kmer_f <<= 2;
            kmer_f |= nuc_1;

            kmer_r <<= 2;
            kmer_r |= nuc_2;

            kmer_g <<= 2;
            kmer_g |= nuc_3;

            kmer_t <<= 2;
            kmer_t |= nuc_4;
        }
        return Some(([kmer_f, kmer_r], [kmer_g, kmer_t]));
    }
}

#[inline]
fn pair_kmer(s1: &[u8], s2: &[u8]) -> Option<([Marker; 2], [Marker; 2])> {
    let k = std::mem::size_of::<Marker>() * 4;
    if s1.len() < 2 * k + 1 || s2.len() < 2 * k + 1 {
        return None;
    } else {
        let mut kmer_f = 0;
        let mut kmer_g = 0;
        let mut kmer_r = 0;
        let mut kmer_t = 0;
        for i in 0..k {
            let nuc_1 = BYTE_TO_SEQ[s1[2 * i] as usize] as Marker;
            let nuc_2 = BYTE_TO_SEQ[s2[2 * i] as usize] as Marker;
            let nuc_3 = BYTE_TO_SEQ[s1[1 + 2 * i] as usize] as Marker;
            let nuc_4 = BYTE_TO_SEQ[s2[1 + 2 * i] as usize] as Marker;

            kmer_f <<= 2;
            kmer_f |= nuc_1;

            kmer_r <<= 2;
            kmer_r |= nuc_2;

            kmer_g <<= 2;
            kmer_g |= nuc_3;

            kmer_t <<= 2;
            kmer_t |= nuc_4;
        }
        return Some(([kmer_f, kmer_r], [kmer_g, kmer_t]));
    }
}

fn dup_removal_lsh_full_exact(
    kmer_counts: &mut FxHashMap<Kmer, u32>,
    kmer_to_pair_set: &mut FxHashSet<(u64, [Marker; 2])>,
    //kmer_to_pair_set: &mut ScalableCuckooFilter<(u64,[Marker;2]), FxHasher>,
    //kmer_to_pair_set: &mut GrowableBloom,
    km: &u64,
    kmer_pair: Option<([Marker; 2], [Marker; 2])>,
    num_dup_removed: &mut usize,
    no_dedup: bool,
    threshold: Option<u32>,
) {
    let c = kmer_counts.entry(*km).or_insert(0);
    let mut c_threshold = u32::MAX;
    if let Some(t) = threshold {
        c_threshold = t;
    }
    if !no_dedup && *c < c_threshold {
        if let Some(doublepairs) = kmer_pair {
            let mut ret = false;
            if kmer_to_pair_set.contains(&(*km, doublepairs.0)) {
                //Need this when using approximate data structures
                if *c > 0 {
                    ret = true;
                }
            } else {
                kmer_to_pair_set.insert((*km, doublepairs.0));
            }
            if kmer_to_pair_set.contains(&(*km, doublepairs.1)) {
                if *c > 0 {
                    ret = true;
                }
            } else {
                kmer_to_pair_set.insert((*km, doublepairs.1));
            }
            if ret {
                *num_dup_removed += 1;
                return;
            }
        }
    }
    *c += 1;
}

fn dup_removal_lsh_full(
    kmer_counts: &mut FxHashMap<Kmer, u32>,
    //kmer_to_pair_set: &mut FxHashSet<(u64,[Marker;2])>,
    kmer_to_pair_set: &mut ScalableCuckooFilter<(u64, [Marker; 2]), FxHasher>,
    //kmer_to_pair_set: &mut GrowableBloom,
    km: &u64,
    kmer_pair: Option<([Marker; 2], [Marker; 2])>,
    num_dup_removed: &mut usize,
    no_dedup: bool,
) {
    let c = kmer_counts.entry(*km).or_insert(0);
    if !no_dedup {
        if let Some(doublepairs) = kmer_pair {
            let mut ret = false;
            if kmer_to_pair_set.contains(&(*km, doublepairs.0)) {
                //Need this when using approximate data structures
                if *c > 0 {
                    ret = true;
                }
            } else {
                kmer_to_pair_set.insert(&(*km, doublepairs.0));
            }
            if kmer_to_pair_set.contains(&(*km, doublepairs.1)) {
                if *c > 0 {
                    ret = true;
                }
            } else {
                kmer_to_pair_set.insert(&(*km, doublepairs.1));
            }
            if ret {
                *num_dup_removed += 1;
                return;
            }
        }
    }
    *c += 1;
}

pub fn sketch_pair_sequences(
    read_file1: &str,
    read_file2: &str,
    c: usize,
    k: usize,
    sample_name: Option<String>,
    no_dedup: bool,
    dedup_fpr: f64,
) -> Option<SequencesSketch> {
    let r1o = parse_fastx_file(&read_file1);
    let r2o = parse_fastx_file(&read_file2);
    let mut read_sketch = SequencesSketch::new(read_file1.to_string(), c, k, true, sample_name, 0.);
    if r1o.is_err() || r2o.is_err() {
        log::error!("Paired end reading failed for '{}' and '{}'. Make sure the files are present or the sequences are valid.", read_file1, read_file2);
        std::process::exit(1);
    }

    let mut num_dup_removed = 0;

    let mut reader1 = r1o.unwrap();
    let mut reader2 = r2o.unwrap();

    //let mut kmer_pair_set = FxHashMap::default();
    let mut kmer_pair_set = FxHashSet::default();
    //let mut kmer_pair_set = GrowableBloom::new(0.001, 1_000_000_0);
    let mut fpr = 0.001;
    if dedup_fpr != 0. {
        fpr = dedup_fpr;
    }
    let mut kmer_pair_set_approx = ScalableCuckooFilterBuilder::new()
        .initial_capacity(1_000_000_0)
        .false_positive_probability(fpr)
        .hasher(FxHasher::default())
        .finish();

    let mut mean_read_length: f64 = 0.;
    let mut counter: f64 = 0.;

    loop {
        let n1 = reader1.next();
        let n2 = reader2.next();
        if let Some(rec1_o) = n1 {
            if let Some(rec2_o) = n2 {
                if let Ok(rec1) = rec1_o {
                    if let Ok(rec2) = rec2_o {
                        let mut temp_vec1 = vec![];
                        let mut temp_vec2 = vec![];

                        extract_markers(&rec1.seq(), &mut temp_vec1, c, k);
                        extract_markers(&rec2.seq(), &mut temp_vec2, c, k);
                        let kmer_pair = pair_kmer(&rec1.seq(), &rec2.seq());

                        //moving average
                        counter += 1.;
                        mean_read_length = mean_read_length
                            + ((rec1.seq().len() as f64) - mean_read_length) / counter;

                        for km in temp_vec1.iter() {
                            if dedup_fpr == 0. {
                                dup_removal_lsh_full_exact(
                                    &mut read_sketch.kmer_counts,
                                    &mut kmer_pair_set,
                                    km,
                                    kmer_pair,
                                    &mut num_dup_removed,
                                    no_dedup,
                                    None,
                                );
                            } else {
                                dup_removal_lsh_full(
                                    &mut read_sketch.kmer_counts,
                                    &mut kmer_pair_set_approx,
                                    km,
                                    kmer_pair,
                                    &mut num_dup_removed,
                                    no_dedup,
                                );
                            }
                            //dup_removal_lsh(&mut read_sketch.kmer_counts, &mut kmer_pair_set, km, kmer_pair, &mut num_dup_removed, no_dedup);
                        }
                        for km in temp_vec2.iter() {
                            if temp_vec1.contains(km) {
                                continue;
                            }
                            if dedup_fpr == 0. {
                                dup_removal_lsh_full_exact(
                                    &mut read_sketch.kmer_counts,
                                    &mut kmer_pair_set,
                                    km,
                                    kmer_pair,
                                    &mut num_dup_removed,
                                    no_dedup,
                                    None,
                                );
                            } else {
                                dup_removal_lsh_full(
                                    &mut read_sketch.kmer_counts,
                                    &mut kmer_pair_set_approx,
                                    km,
                                    kmer_pair,
                                    &mut num_dup_removed,
                                    no_dedup,
                                );
                            }
                            //dup_removal_lsh(&mut read_sketch.kmer_counts, &mut kmer_pair_set, km, kmer_pair, &mut num_dup_removed, no_dedup);
                        }
                    }
                } else {
                    return None;
                }
            }
        } else {
            break;
        }
    }
    let num_kmers = read_sketch.kmer_counts.values().sum::<u32>() as f64;
    let percent = (num_dup_removed as f64)/((read_sketch.kmer_counts.values().sum::<u32>() as f64) + num_dup_removed as f64) * 100.;
    log::debug!(
        "Number of sketched k-mers removed due to read duplication for {}: {}. Percentage: {:.2}%",
        read_sketch.file_name,
        num_dup_removed,
        percent,
    );
    read_sketch.mean_read_length = mean_read_length;
    return Some(read_sketch);
}

pub fn sketch_sequences_needle(
    read_file: &str,
    c: usize,
    k: usize,
    sample_name: Option<String>,
    no_dedup: bool,
) -> Option<SequencesSketch> {
    let mut kmer_map = HashMap::default();
    let ref_file = &read_file;
    let reader = parse_fastx_file(&ref_file);
    let mut mean_read_length = 0.;
    let mut counter = 0.;
    let mut kmer_to_pair_table = FxHashSet::default();
    let mut num_dup_removed = 0;

    if !reader.is_ok() {
        warn!("{} is not a valid fasta/fastq file; skipping.", ref_file);
    } else {
        let mut reader = reader.unwrap();
        while let Some(record) = reader.next() {
            if record.is_ok() {
                let mut vec = vec![];
                let record = record.expect(&format!("Invalid record for file {} ", ref_file));
                let seq = record.seq();
                let kmer_pair;
                if seq.len() > 400 {
                    kmer_pair = None;
                } else {
                    kmer_pair = pair_kmer_single(&seq);
                }
                extract_markers(&seq, &mut vec, c, k);
                for km in vec {
                    dup_removal_lsh_full_exact(
                        &mut kmer_map,
                        &mut kmer_to_pair_table,
                        &km,
                        kmer_pair,
                        &mut num_dup_removed,
                        no_dedup,
                        Some(MAX_DEDUP_COUNT),
                    );
                }
                //moving average
                counter += 1.;
                mean_read_length =
                    mean_read_length + ((seq.len() as f64) - mean_read_length) / counter;
            } else {
                warn!("File {} is not a valid fasta/fastq file", ref_file);
            }
        }
    }

    return Some(SequencesSketch {
        kmer_counts: kmer_map,
        file_name: read_file.to_string(),
        c,
        k,
        paired: false,
        sample_name,
        mean_read_length,
    });
}
