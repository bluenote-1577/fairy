use crate::cmdline::*;
use human_sort::sort;
use std::path::Path;
use std::io::prelude::*;
use std::io;
use std::io::BufWriter;
use fxhash::FxHashMap;
use fxhash::FxHashSet;
use crate::constants::*;
use crate::inference::*;
use crate::sketch::*;
use crate::types::*;
use log::*;
use rayon::prelude::*;
use statrs::distribution::{DiscreteCDF, Poisson};
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;

fn print_cov_matrix(ani_results: Vec<AniResult>, read_files: &FxHashSet<String>, genome_sketches: &Vec<GenomeSketch>, writer: &mut Box<dyn Write + Send>, args: &ContainArgs) {

    let mut matrix: FxHashMap<&str, FxHashMap<&str,(f64,f64)>> = FxHashMap::default();
    for res in ani_results.iter(){
        let inner = matrix.entry(res.contig_name).or_insert(FxHashMap::default());
        inner.insert(&res.seq_name, (res.final_est_cov, res.var));
    }
    //debug!("number of contigs processed {}", ani_results.len());
    //dbg!(&matrix);

    let contig_list_sorted = genome_sketches.iter().map(|x| x.first_contig_name.as_str()).collect::<Vec<&str>>();
    //dbg!(&contig_list_sorted[0..10]);
    let contig_to_size = genome_sketches.iter().map(|x| (x.first_contig_name.as_str(), x.gn_size)).collect::<FxHashMap<&str, usize>>();
    let mut read_list_sorted = read_files.iter().map(|x| x.as_str()).collect::<Vec<&str>>();

    //sort(&mut contig_list_sorted);
    sort(&mut read_list_sorted);

    if args.concoct_format{
        write!(writer, "contigName").unwrap();
        for read_name in read_list_sorted.iter(){
            write!(writer, "\t{}", read_name).unwrap();
        }
    }
    else{
        write!(writer, "contigName\tcontigLen\ttotalAvgDepth").unwrap();
        for read_name in read_list_sorted.iter(){
            write!(writer, "\t{}\t{}-var", read_name, read_name).unwrap();
        }
    }
    write!(writer, "\n").unwrap();
    for contig in contig_list_sorted{
        write!(writer, "{}", contig.split(' ').collect::<Vec<&str>>()[0]).unwrap();
        if !args.concoct_format{
            write!(writer, "\t{}", contig_to_size[contig]).unwrap();
        }
        let mut avg_cov = 0.;
        if matrix.contains_key(contig){
            for value in matrix[contig].values(){
                avg_cov += value.0;
            }
        }
        else{
        }
        avg_cov /= read_list_sorted.len() as f64;
        if !args.concoct_format{
            write!(writer, "\t{}", avg_cov).unwrap();
        }
        for read in read_list_sorted.iter(){
            if !args.concoct_format{
                if matrix.contains_key(contig) && matrix[contig].contains_key(read){
                    let (cov, var) = matrix[&contig][read];
                    write!(writer, "\t{}\t{}", cov, var).unwrap();
                }
                else{
                    write!(writer, "\t0\t0").unwrap();
                }
            }
            else{
                if matrix.contains_key(contig) && matrix[contig].contains_key(read){
                    let (cov, _var) = matrix[&contig][read];
                    write!(writer, "\t{}", cov).unwrap();
                }
                else{
                    write!(writer, "\t0").unwrap();
                }

            }
        }
        write!(writer, "\n").unwrap();
    }

}

fn _print_ani_result(ani_result: &AniResult, pseudotax: bool, writer: &mut Box<dyn Write + Send>) {
    let print_final_ani = format!("{:.2}", f64::min(ani_result.final_est_ani * 100., 100.));
    let lambda_print;
    if let AdjustStatus::Lambda(lambda) = ani_result.lambda {
        lambda_print = format!("{:.3}", lambda);
    } else if ani_result.lambda == AdjustStatus::High {
        lambda_print = format!("HIGH");
    } else {
        lambda_print = format!("LOW");
    }
    let low_ani = ani_result.ani_ci.0;
    let high_ani = ani_result.ani_ci.1;
    let low_lambda = ani_result.lambda_ci.0;
    let high_lambda = ani_result.lambda_ci.1;

    let ci_ani;
    if low_ani.is_none() || high_ani.is_none() {
        ci_ani = "NA-NA".to_string();
    } else {
        ci_ani = format!(
            "{:.2}-{:.2}",
            low_ani.unwrap() * 100.,
            high_ani.unwrap() * 100.
        );
    }

    let ci_lambda;
    if low_lambda.is_none() || high_lambda.is_none() {
        ci_lambda = "NA-NA".to_string();
    } else {
        ci_lambda = format!("{:.2}-{:.2}", low_lambda.unwrap(), high_lambda.unwrap());
    }


    //"Sample_file\tQuery_file\tTaxonomic_abundance\tSequence_abundance\tAdjusted_ANI\tEff_cov\tANI_5-95_percentile\tEff_lambda\tLambda_5-95_percentile\tMedian_cov\tMean_cov_geq1\tContainment_ind\tNaive_ANI\tContig_name",

    if !pseudotax{
        writeln!(writer, 
            "{}\t{}\t{}\t{:.3}\t{}\t{}\t{}\t{:.0}\t{:.3}\t{}/{}\t{:.2}\t{}",
            ani_result.seq_name,
            ani_result.gn_name,
            print_final_ani,
            ani_result.final_est_cov,
            ci_ani,
            lambda_print,
            ci_lambda,
            ani_result.median_cov,
            ani_result.mean_cov,
            ani_result.containment_index.0,
            ani_result.containment_index.1,
            ani_result.naive_ani * 100.,
            ani_result.contig_name,
        ).expect("Error writing to file");
    }
    else{
        writeln!(writer,
            "{}\t{}\t{:.4}\t{:.4}\t{}\t{:.3}\t{}\t{}\t{}\t{:.0}\t{:.3}\t{}/{}\t{:.2}\t{}",
            ani_result.seq_name,
            ani_result.gn_name,
            ani_result.rel_abund.unwrap(),
            ani_result.seq_abund.unwrap(),
            print_final_ani,
            ani_result.final_est_cov,
            ci_ani,
            lambda_print,
            ci_lambda,
            ani_result.median_cov,
            ani_result.mean_cov,
            ani_result.containment_index.0,
            ani_result.containment_index.1,
            ani_result.naive_ani * 100.,
            ani_result.contig_name,
        ).expect("Error writing to file");

    }
}

fn get_chunks(indices: &Vec<usize>, steps: usize) -> Vec<Vec<usize>>{
    let mut start = 0;
    let mut end = steps;
    let len = indices.len();
    let mut return_chunks = vec![];

    while start < len {
        if end > len {
            end = len;
        }

        let chunk: Vec<usize> = (start..end).collect();
        start = end;
        end += steps;
        return_chunks.push(chunk);
    }
    return_chunks
}

pub fn contain(mut args: ContainArgs, pseudotax_in: bool) {

    if pseudotax_in{
        args.pseudotax = true;
    }

    let level;
    if args.trace {
        level = log::LevelFilter::Trace;
    } else if args.debug {
        level = log::LevelFilter::Debug;
    }
    else{
        level = log::LevelFilter::Info;
    }
    
    simple_logger::SimpleLogger::new()
        .with_level(level)
        .init()
        .unwrap();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    let mut out_writer = match args.out_file_name {
        Some(ref x) => {
            let path = Path::new(&x);
            Box::new(BufWriter::new(File::create(&path).unwrap())) as Box<dyn Write + Send>
        }
        None => Box::new(BufWriter::new(io::stdout())) as Box<dyn Write + Send>,
    };

    log::info!("Obtaining sketches...");
    let mut genome_sketch_files = vec![];
    let mut genome_files = vec![];
    let mut read_sketch_files = vec![];
    let mut read_files = vec![];

    let mut all_files = args.files.clone();

    if let Some(ref newline_file) = args.file_list{
        let file = File::open(newline_file).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            all_files.push(line.unwrap());
        }

    }

    for file in all_files.iter(){

        let mut genome_sketch_good_suffix = false;
        for suff in QUERY_FILE_SUFFIX_VALID{
            if file.ends_with(suff){
                genome_sketch_good_suffix = true;
                break
            }
        }

        let mut sample_sketch_good_suffix = false;
        for suff in SAMPLE_FILE_SUFFIX_VALID{
            if file.ends_with(suff){
                sample_sketch_good_suffix = true;
                break
            }
        }

        if genome_sketch_good_suffix{
            genome_sketch_files.push(file);
        } else if sample_sketch_good_suffix{
            read_sketch_files.push(file);
        } else if is_fasta(&file) {
            genome_files.push(file);
        } else if is_fastq(&file) {
            read_files.push(file);
        } else {
            warn!(
                "{} file extension is not a sketch or a fasta/fastq file.",
                &file
            );
        }
    }

    if genome_sketch_files.is_empty() && genome_files.is_empty(){
        log::error!("No contigs files found; see fairy coverage -h for help. Exiting");
        std::process::exit(1);
    }

    if read_sketch_files.is_empty() && read_files.is_empty(){
        log::error!("No read files found; see fairy coverage -h for help. Exiting");
        std::process::exit(1);
    }

    let genome_sketches_vec = get_genome_sketches(&args, &genome_sketch_files, &genome_files);
    log::info!("Finished indexing contigs.");

    if genome_sketches_vec.is_empty() {
        log::error!("No contigs found; see fariy coverage -h for help. Exiting");
        std::process::exit(1);
    }

    let step;
    if let Some(sample_threads) = args.sample_threads{
        if sample_threads > 0{
            step = sample_threads;
        }
        else{
            step = 1;
        }
    }
    else{
        if args.pseudotax{
            step = args.threads/2 + 1;
        }
        else{
            step = 1
        }
    }

    read_files.extend(read_sketch_files.clone());
    let sequence_index_vec = (0..read_files.len()).collect::<Vec<usize>>();

    for genome_sketches in genome_sketches_vec {
        let chunks = get_chunks(&sequence_index_vec, step);
        let stats_vec_seq_all: Mutex<Vec<AniResult>> = Mutex::new(vec![]);
        let sequence_file_names: Mutex<FxHashSet<String>> = Mutex::new(FxHashSet::default());
        chunks.into_iter().for_each(|chunk| {
            chunk.into_par_iter().for_each(|j|{
                
                let is_sketch;
                if read_files[j].ends_with(SAMPLE_FILE_SUFFIX){
                    is_sketch = true;
                }
                else{
                    is_sketch = false;
                }
                let stats_vec_seq = Mutex::new(vec![]);
                let c;
                let k;
                if genome_sketches.is_empty(){
                    c = 50;
                    k = 31;
                }
                else{
                    c = genome_sketches[0].c;
                    k = genome_sketches[0].k;
                }
                let sequence_sketch = get_seq_sketch(&args, read_files[j], is_sketch, c, k);
                if sequence_sketch.is_some(){

                    {
                        let seq_sketch = sequence_sketch.as_ref().unwrap();
                        if let Some(sample) = &seq_sketch.sample_name{
                            sequence_file_names.lock().unwrap().insert(sample.clone());
                        }
                        else{
                            sequence_file_names.lock().unwrap().insert(seq_sketch.file_name.clone());
                        }
                    }

                    let sequence_sketch = sequence_sketch.unwrap();
//                    if sequence_sketch.mean_read_length > 1000. && args.seq_id > 99. {
                        //log::info!("Long reads detected and --read-seq-id >= 99. If using older, error prone reads, set --read-seq-id lower for slightly better results.");
//                    }

                    let kmer_id_opt;
                    kmer_id_opt = Some((args.seq_id/100.).powf(sequence_sketch.k as f64));
                    log::debug!("{} has estimated kmer identity {:.3}.", &read_files[j], kmer_id_opt.unwrap());
                    if kmer_id_opt.is_none(){
                        log::error!("Something went wrong with read sequence identity processing for {}. Exiting",  &read_files[j]);
                        std::process::exit(1);
                    }
                    
                    let genome_index_vec = (0..genome_sketches.len()).into_iter().collect::<Vec<usize>>();
                    genome_index_vec.par_iter().for_each(|i| {
                        let genome_sketch = &genome_sketches[*i];
                        let res = get_stats(&args, &genome_sketch, &sequence_sketch, None);
                        if res.is_some() {
                            //res.as_mut().unwrap().genome_sketch_index = *i;
                            //stats_vec_seq.lock().unwrap().entry(res.unwrap().seq_name).or_insert(vec![]).push(res.unwrap());
                            stats_vec_seq.lock().unwrap().push(res.unwrap());
                        }
                    });

                    let mut stats_vec_seq = stats_vec_seq.into_inner().unwrap();

                    estimate_true_cov(&mut stats_vec_seq, kmer_id_opt, true, sequence_sketch.mean_read_length, sequence_sketch.k);
                    let bases_explained;
                    bases_explained = estimate_covered_bases(&stats_vec_seq, &sequence_sketch, sequence_sketch.mean_read_length, sequence_sketch.k);
                    if !stats_vec_seq.is_empty(){
                        log::info!("{} has approximately {:.2}% of reads detected 
                            in contigs {} (only accurate for low-error reads)", 
                            &read_files[j], 
                            bases_explained * 100., 
                            &stats_vec_seq[0].genome_sketch.file_name);
                    }

                    log::info!("{} reassigning k-mers for {} contigs...", &read_files[j], stats_vec_seq.len());
                    let winner_map = winner_table(&stats_vec_seq);
                    let remaining_genomes = stats_vec_seq.iter().map(|x| x.genome_sketch).collect::<Vec<&GenomeSketch>>();
                    let stats_vec_seq_2 = Mutex::new(vec![]);
                    remaining_genomes.into_par_iter().for_each(|genome_sketch|{
                        let res = get_stats(&args, &genome_sketch, &sequence_sketch, Some(&winner_map));
                        if res.is_some() {
                            stats_vec_seq_2.lock().unwrap().push(res.unwrap());
                        }
                    });
                    //stats_vec_seq = derep_if_reassign_threshold(&stats_vec_seq, stats_vec_seq_2.into_inner().unwrap(), args.redundant_ani, sequence_sketch.k);
                    //estimate_true_cov(&mut stats_vec_seq, kmer_id_opt, true, sequence_sketch.mean_read_length, sequence_sketch.k);
                    stats_vec_seq = stats_vec_seq_2.into_inner().unwrap();
                    log::info!("{} has {} contigs passing ANI threshold. ", &read_files[j], stats_vec_seq.len());

                    stats_vec_seq_all.lock().unwrap().extend(stats_vec_seq);
                }
                log::info!("Finished sample {}.", &read_files[j]);
            });
        });
        print_cov_matrix(stats_vec_seq_all.into_inner().unwrap(), &sequence_file_names.into_inner().unwrap(), &genome_sketches,&mut out_writer, &args);
    }

    log::info!("fairy finished.");
}

fn _derep_if_reassign_threshold<'a>(results_old: &Vec<AniResult>, results_new: Vec<AniResult<'a>>, ani_thresh: f64, k: usize) -> Vec<AniResult<'a>>{
    let ani_thresh = ani_thresh/100.;

    let mut gn_sketch_to_contain = FxHashMap::default();
    for result in results_old.iter(){
        gn_sketch_to_contain.insert(result.genome_sketch, result);
    }

    let threshold = f64::powf(ani_thresh, k as f64);
    let mut return_vec = vec![];
    for result in results_new.into_iter(){
        let old_res = &gn_sketch_to_contain[result.genome_sketch];
        let num_kmer_reassign = (old_res.containment_index.0 - result.containment_index.0) as f64;
        let reass_thresh = threshold * result.containment_index.1 as f64;
        if num_kmer_reassign < reass_thresh{
            return_vec.push(result);
        }
        else{
            log::debug!("genome {} had num k-mers reassigned = {}, threshold was {}, removing.", result.gn_name, num_kmer_reassign, reass_thresh);
        }
    }
    return return_vec;
}

fn estimate_true_cov(results: &mut Vec<AniResult>, kmer_id_opt: Option<f64>, 
                     estimate_unknown: bool, read_length: f64, k: usize){
    let mut multiplier = 1.;
    if estimate_unknown{
        multiplier = read_length / (read_length - k as f64 + 1.);
    }
    if estimate_unknown && kmer_id_opt.is_some(){
        let id = kmer_id_opt.unwrap();
        for res in results.iter_mut(){
            res.final_est_cov = res.final_est_cov / id * multiplier ;
        }
    }
}

fn estimate_covered_bases(results: &Vec<AniResult>, sequence_sketch: &SequencesSketch, read_length: f64, k: usize) -> f64{
    let multiplier = read_length / (read_length - (k as f64) + 1.);

    let mut num_covered_bases = 0.;
    for res in results.iter(){
        num_covered_bases += (res.genome_sketch.gn_size as f64) * res.final_est_cov
    }
    let mut num_total_counts = 0;
    for count in sequence_sketch.kmer_counts.values(){
        num_total_counts += *count as usize;
    }
    let num_tentative_bases = sequence_sketch.c * num_total_counts;
    let num_tentative_bases = num_tentative_bases as f64 * multiplier;
    if num_tentative_bases == 0.{
        return 0.;
    }
    return f64::min(num_covered_bases as f64 / num_tentative_bases, 1.);
}

fn winner_table<'a>(results : &'a Vec<AniResult>) -> FxHashMap<Kmer, (f64,&'a GenomeSketch, bool)> {
    let mut kmer_to_genome_map : FxHashMap<_,_> = FxHashMap::default();
    for res in results.iter(){
        //let gn_sketch = &genome_sketches[res.genome_sketch_index];
        let gn_sketch = res.genome_sketch;
        for kmer in gn_sketch.genome_kmers.iter(){
            let v = kmer_to_genome_map.entry(*kmer).or_insert((res.final_est_ani, res.genome_sketch, false));
            if res.final_est_ani > v.0{
                *v = (res.final_est_ani, gn_sketch, true);
            }
        }
        
        if gn_sketch.pseudotax_tracked_nonused_kmers.is_some(){
            for kmer in gn_sketch.pseudotax_tracked_nonused_kmers.as_ref().unwrap().iter(){
                let v = kmer_to_genome_map.entry(*kmer).or_insert((res.final_est_ani, res.genome_sketch, false));
                if res.final_est_ani > v.0{
                    *v = (res.final_est_ani, gn_sketch, true);
                }
            }
        }
    }

    return kmer_to_genome_map;
}

fn _print_header(pseudotax: bool, writer: &mut Box<dyn Write + Send>, estimate_unknown: bool) {
    if !pseudotax{
        writeln!(writer,
            //"Sample_file\tQuery_file\tAdjusted_ANI\tNaive_ANI\tANI_5-95_percentile\tEff_cov\tEff_lambda\tLambda_5-95_percentile\tMedian_cov\tMean_cov_geq1\tContainment_ind\tContig_name",
            "Sample_file\tGenome_file\tAdjusted_ANI\tEff_cov\tANI_5-95_percentile\tEff_lambda\tLambda_5-95_percentile\tMedian_cov\tMean_cov_geq1\tContainment_ind\tNaive_ANI\tContig_name",
            ).expect("Error writing to file.");
    }
    else{
        let cov_head;
        if estimate_unknown{
            cov_head = "True_cov";
        }
        else{
            cov_head = "Eff_cov";
        }
        writeln!(writer,
            "Sample_file\tGenome_file\tTaxonomic_abundance\tSequence_abundance\tAdjusted_ANI\t{}\tANI_5-95_percentile\tEff_lambda\tLambda_5-95_percentile\tMedian_cov\tMean_cov_geq1\tContainment_ind\tNaive_ANI\tContig_name", cov_head
            ).expect("Error writing to file.");
    }
}

fn get_genome_sketches(
    args: &ContainArgs,
    genome_sketch_files: &Vec<&String>,
    genome_files: &Vec<&String>,
) -> Vec<Vec<GenomeSketch>> {
    let mut lowest_genome_c = None;
    let mut current_k = None;

    let genome_sketches = Mutex::new(vec![]);

    for genome_sketch_file in genome_sketch_files {
        let file = File::open(genome_sketch_file).expect(&format!("The sketch `{}` could not be opened. Exiting", genome_sketch_file));
        let genome_reader = BufReader::with_capacity(10_000_000, file);
        let genome_sketches_vec: Vec<GenomeSketch> = bincode::deserialize_from(genome_reader)
            .expect(&format!(
                "The sketch `{}` is not a valid sketch. Perhaps it is an older, incompatible version ",
                &genome_sketch_file
            ));
        if genome_sketches_vec.is_empty() {
            continue;
        }
        let c = genome_sketches_vec.first().unwrap().c;
        let k = genome_sketches_vec.first().unwrap().k;
        if lowest_genome_c.is_none() {
            lowest_genome_c = Some(c);
        } else if lowest_genome_c.unwrap() < c {
            lowest_genome_c = Some(c);
        }
        if current_k.is_none() {
            current_k = Some(genome_sketches_vec.first().unwrap().k);
        } else if current_k.unwrap() != k {
            error!("Query sketches have inconsistent -k. Exiting.");
            std::process::exit(1);
        }
        genome_sketches.lock().unwrap().push(genome_sketches_vec);
    }

    genome_files.into_par_iter().for_each(|genome_file|{
        if lowest_genome_c.is_some() && lowest_genome_c.unwrap() < args.c{
            error!("Value of -c for contain is {} -- greater than the smallest value of -c for a genome sketch {}. Continuing without sketching.", args.c, lowest_genome_c.unwrap());
        }
        else if current_k.is_some() && current_k.unwrap() != args.k{
            error!("-k {} is not equal to -k {} found in sketches. Continuing without sketching.", args.k, current_k.unwrap());
        }
        else {
            let indiv_gn_sketches = sketch_genome_individual(args.c, args.k, genome_file, args.min_spacing_kmer, args.pseudotax);
            genome_sketches.lock().unwrap().push(indiv_gn_sketches);
        }
    });

    return genome_sketches.into_inner().unwrap();
}

fn get_seq_sketch(
    args: &ContainArgs,
    read_file: &str,
    is_sketch_file: bool,
    genome_c: usize,
    genome_k: usize,
) -> Option<SequencesSketch> {
    if is_sketch_file {
        let read_sketch_file = read_file;
        let file = File::open(read_sketch_file).expect(&format!(
            "The sketch `{}` could not be opened. Make sure it exists. Exiting.",
            &read_sketch_file
        ));
        let read_reader = BufReader::with_capacity(10_000_000, file);
        let read_sketch_enc: SequencesSketchEncode = bincode::deserialize_from(read_reader).expect(
            &format!("The sketch `{}` is not a valid sketch. Perhaps it is an older incompatible version ", read_sketch_file),
        );
        let read_sketch = SequencesSketch::from_enc(read_sketch_enc);
        if read_sketch.c > genome_c {
            error!("{} value of -c for {} is {} -- greater than the smallest value of -c for a genome sketch {}. Exiting.", read_file, read_sketch.c, read_sketch_file, genome_c);
            return None;
        }

        return Some(read_sketch);
    } else {
        if genome_c < args.c {
            error!("{} error: value of -c for contain = {} -- greater than the smallest value of -c for a genome sketch = {}. Continuing without sketching.", read_file, args.c, genome_c);
            return None;
        } else if genome_k != args.k {
            error!(
                "{} -k {} is not equal to -k {} found in sketches. Continuing without sketching.",
                read_file, args.k, genome_k
            );
            return None;
        } else {
            let read_sketch_opt = sketch_sequences_needle(&read_file, args.c, args.k, None, false, );
            return read_sketch_opt;
        }
    }
}

fn _get_sketches_rewrite(args: &ContainArgs) -> (Vec<SequencesSketch>, Vec<GenomeSketch>) {
    let mut read_sketch_files = vec![];
    let mut genome_sketch_files = vec![];
    let mut read_files = vec![];
    let mut genome_files = vec![];
    for file in args.files.iter() {
        if file.ends_with(QUERY_FILE_SUFFIX) {
            genome_sketch_files.push(file);
        } else if file.ends_with(SAMPLE_FILE_SUFFIX) {
            read_sketch_files.push(file);
        } else if is_fasta(&file) {
            genome_files.push(file);
        } else if is_fastq(&file) {
            read_files.push(file);
        } else {
            warn!(
                "{} file extension is not a sketch or a fasta/fastq file.",
                &file
            );
        }
    }

    let genome_sketches = Mutex::new(vec![]);
    let read_sketches = Mutex::new(vec![]);
    //read c can be lower than lowest genome c.
    let mut lowest_genome_c = None;
    let mut current_k = None;

    read_sketch_files.into_par_iter().for_each(|read_sketch_file|{
        let file = File::open(read_sketch_file.clone()).expect(&format!("The sketch `{}` could not be opened. Exiting ", &read_sketch_file));
        let read_reader = BufReader::with_capacity(10_000_000, file);
        let read_sketch_enc: SequencesSketchEncode = bincode::deserialize_from(read_reader).expect(&format!(
            "The sketch `{}` is not a valid sketch. It is either corrupted or an older incompatible version ",
            read_sketch_file
        ));
        let read_sketch = SequencesSketch::from_enc(read_sketch_enc);
        if lowest_genome_c.is_some() && read_sketch.c > lowest_genome_c.unwrap(){
            error!("Value of -c for {} is {} -- greater than the smallest value of -c for a genome sketch {}. Exiting.", read_sketch.c, read_sketch_file, lowest_genome_c.unwrap());
            std::process::exit(1);
        }
        read_sketches.lock().unwrap().push(read_sketch);
    });

    for genome_sketch_file in genome_sketch_files {
        let file =
            File::open(genome_sketch_file.clone()).expect(&format!("The sketch `{}` could not be opened. Exiting ", genome_sketch_file));
        let genome_reader = BufReader::with_capacity(10_000_000, file);
        let genome_sketches_vec: Vec<GenomeSketch> = bincode::deserialize_from(genome_reader)
            .expect(&format!(
                "The sketch `{}` is not a valid sketch. It is either corrupted or an older incompatible version ",
                &genome_sketch_file
            ));
        if genome_sketches_vec.is_empty() {
            continue;
        }
        let c = genome_sketches_vec.first().unwrap().c;
        let k = genome_sketches_vec.first().unwrap().k;
        if lowest_genome_c.is_none() {
            lowest_genome_c = Some(c);
        } else if lowest_genome_c.unwrap() < c {
            lowest_genome_c = Some(c);
        }
        if current_k.is_none() {
            current_k = Some(genome_sketches_vec.first().unwrap().k);
        } else if current_k.unwrap() != k {
            error!("Query sketches have inconsistent -k. Exiting.");
            std::process::exit(1);
        }
        genome_sketches.lock().unwrap().extend(genome_sketches_vec);
    }

    genome_files.into_par_iter().for_each(|genome_file|{
        if lowest_genome_c.is_some() && lowest_genome_c.unwrap() < args.c{
            error!("Value of -c for contain is {} -- greater than the smallest value of -c for a genome sketch {}. Continuing without sketching.", args.c, lowest_genome_c.unwrap());
        }
        else if current_k.is_some() && current_k.unwrap() != args.k{
            error!("-k {} is not equal to -k {} found in sketches. Continuing without sketching.", args.k, current_k.unwrap());
        }
        else {
            let indiv_gn_sketches = sketch_genome_individual(args.c, args.k, genome_file, args.min_spacing_kmer, args.pseudotax);
            genome_sketches.lock().unwrap().extend(indiv_gn_sketches);
        }
    });

    read_files.into_par_iter().for_each(|read_file|{
        if lowest_genome_c.is_some() && lowest_genome_c.unwrap() < args.c{
            error!("Value of -c for contain is {} -- greater than the smallest value of -c for a genome sketch {}. Continuing without sketching.", args.c, lowest_genome_c.unwrap());
        }
        else if current_k.is_some() && current_k.unwrap() != args.k{
            error!("-k {} is not equal to -k {} found in sketches. Continuing without sketching.", args.k, current_k.unwrap());
        }
        else {
            let read_sketch_opt = sketch_sequences_needle(&read_file,args.c, args.k, None, false);
            if read_sketch_opt.is_some() {
                read_sketches.lock().unwrap().push(read_sketch_opt.unwrap());
            }
        }
    });

    return (
        read_sketches.into_inner().unwrap(),
        genome_sketches.into_inner().unwrap(),
    );
}

fn get_stats<'a>(
    args: &ContainArgs,
    genome_sketch: &'a GenomeSketch,
    sequence_sketch: &SequencesSketch,
    winner_map: Option<&FxHashMap<Kmer, (f64,& GenomeSketch, bool)>>
) -> Option<AniResult<'a>> {
    if genome_sketch.k != sequence_sketch.k {
        log::error!(
            "k parameter for reads {} != k parameter for genome {}",
            sequence_sketch.k,
            genome_sketch.k
        );
        std::process::exit(1);
    }
    if genome_sketch.c < sequence_sketch.c {
        log::error!(
            "c parameter for reads {} > c parameter for genome {}",
            sequence_sketch.c,
            genome_sketch.c
        );
        std::process::exit(1);
    }
    let mut contain_count = 0;
    let mut covs = vec![];
    let gn_kmers = &genome_sketch.genome_kmers;
    if (gn_kmers.len() as f64) < args.min_number_kmers{
        return None
    }

    //let start_t_initial = Instant::now();
    for kmer in gn_kmers.iter() {
        if sequence_sketch.kmer_counts.contains_key(kmer) {
            if sequence_sketch.kmer_counts[kmer] == 0{
                continue
            }
            if winner_map.is_some(){
                let map = &winner_map.unwrap();
                //TODO... algorithm testing
                if map[kmer].1 != genome_sketch{// || map[kmer].2 {
                    continue
                }
                contain_count += 1;
                covs.push(sequence_sketch.kmer_counts[kmer]);

            }
            else{
                contain_count += 1;
                covs.push(sequence_sketch.kmer_counts[kmer]);
            }
        }
    }
    //log::trace!("Hashing time {:?}", Instant::now() - start_t_initial);
    //let start_t_initial = Instant::now();
    if covs.is_empty() {
        return None;
    }
    let naive_ani = f64::powf(
        contain_count as f64 / gn_kmers.len() as f64,
        1. / genome_sketch.k as f64,
    );
    covs.sort();
    //let covs = &covs[0..covs.len() * 99 / 100];
    let median_cov = covs[covs.len() / 2] as f64;
    let pois = Poisson::new(median_cov).unwrap();
    let mut max_cov = f64::MAX;
    if median_cov < 30.{
        for i in covs.len() / 2..covs.len(){
            let cov = covs[i];
            if pois.cdf(cov.into()) < CUTOFF_PVALUE {
                max_cov = cov as f64;
            } else {
                break;
            }
        }
    }

    log::trace!("COV VECTOR for {}/{}: {:?}, MAX_COV_THRESHOLD: {}", sequence_sketch.file_name, genome_sketch.first_contig_name, covs, max_cov);

    let mut full_covs = vec![0; gn_kmers.len() - contain_count];
    for cov in covs.iter() {
        if (*cov as f64) <= max_cov {
            full_covs.push(*cov);
        }
    }
    let var_opt = var(&full_covs);
    if var_opt.is_some(){
        log::trace!("VAR {} {}", var_opt.unwrap(), genome_sketch.file_name);
    }
    let mean_cov = full_covs.iter().sum::<u32>() as f64 / full_covs.len() as f64;
    let geq1_mean_cov = full_covs.iter().sum::<u32>() as f64 / covs.len() as f64;

    let use_lambda;
    if median_cov > MEDIAN_ANI_THRESHOLD {
        use_lambda = AdjustStatus::High
    } else {
        let test_lambda;
        if args.ratio {
            test_lambda = ratio_lambda(&full_covs, args.min_count_correct)
        } else if args.mme {
            test_lambda = mme_lambda(&full_covs)
        } else if args.nb {
            test_lambda = binary_search_lambda(&full_covs)
        } else if args.mle {
            test_lambda = mle_zip(&full_covs, sequence_sketch.k as f64)
        } else {
            test_lambda = ratio_lambda(&full_covs, args.min_count_correct)
        };
        if test_lambda.is_none() {
            use_lambda = AdjustStatus::Low
        } else {
            use_lambda = AdjustStatus::Lambda(test_lambda.unwrap());
        }
    }

    let final_est_cov;

    if let AdjustStatus::Lambda(lam) = use_lambda {
        final_est_cov = lam
    } else if median_cov < MAX_MEDIAN_FOR_MEAN_FINAL_EST{
        final_est_cov = geq1_mean_cov;
    } else{
        final_est_cov = median_cov;
    }

    let opt_lambda;
    if use_lambda == AdjustStatus::Low || use_lambda == AdjustStatus::High {
        opt_lambda = None
    } else {
        opt_lambda = Some(final_est_cov)
    };

    let opt_est_ani = ani_from_lambda(opt_lambda, mean_cov, sequence_sketch.k as f64, &full_covs);
    
    let final_est_ani;
    if opt_lambda.is_none() || opt_est_ani.is_none() || args.no_adj {
        final_est_ani = naive_ani;
    } else {
        final_est_ani = opt_est_ani.unwrap();
    }

    let min_ani = if args.minimum_ani.is_some() {args.minimum_ani.unwrap()/100. }
        else if args.pseudotax { MIN_ANI_P_DEF } 
        else { MIN_ANI_DEF };
    if final_est_ani < min_ani {
        return None;
    }

    let (mut low_ani, mut high_ani, mut low_lambda, mut high_lambda) = (None, None, None, None);
    if !args.no_ci && opt_lambda.is_some() {
        let bootstrap = bootstrap_interval(&full_covs, sequence_sketch.k as f64, &args);
        low_ani = bootstrap.0;
        high_ani = bootstrap.1;
        low_lambda = bootstrap.2;
        high_lambda = bootstrap.3;
    }

    
    let seq_name;
    if let Some(sample) = &sequence_sketch.sample_name{
        seq_name = sample.clone();
    }
    else{
        seq_name = sequence_sketch.file_name.clone();
    }

    let var;
    if var_opt.is_some(){
        var = var_opt.unwrap();
    }
    else
    {
        return None;
    }
    let ani_result = AniResult {
        naive_ani,
        final_est_ani,
        final_est_cov,
        seq_name,
        gn_name: genome_sketch.file_name.as_str(),
        contig_name: genome_sketch.first_contig_name.as_str(),
        mean_cov: geq1_mean_cov,
        median_cov,
        containment_index: (contain_count, gn_kmers.len()),
        lambda: use_lambda,
        ani_ci: (low_ani, high_ani),
        lambda_ci: (low_lambda, high_lambda),
        genome_sketch,
        rel_abund: None,
        seq_abund: None,
        var: var as f64,
    };
    //log::trace!("Other time {:?}", Instant::now() - start_t_initial);

    return Some(ani_result);
}

fn _ani_from_lambda_moment(lambda: Option<f64>, mean: f64, k: f64) -> Option<f64> {
    if lambda.is_none() {
        return None;
    }
    let lambda = lambda.unwrap();
    let pi = ((lambda + 1.) * mean - mean * mean - mean) / ((lambda + 1.) * mean - mean);
    let ret_ani;
    let ani = f64::powf(1. - pi, 1. / k);
    if ani < 0. || ani.is_nan() {
        ret_ani = None;
    } else {
        if ani > 1. {
            ret_ani = Some(1.)
        } else {
            ret_ani = Some(ani);
        }
    }
    return ret_ani;
}

fn ani_from_lambda(lambda: Option<f64>, _mean: f64, k: f64, full_cov: &[u32]) -> Option<f64> {
    if lambda.is_none() {
        return None;
    }
    let mut contain_count = 0;
    let mut _zero_count = 0;
    for x in full_cov {
        if *x != 0 {
            contain_count += 1;
        } else {
            _zero_count += 1;
        }
    }

    let lambda = lambda.unwrap();
    let adj_index =
        contain_count as f64 / (1. - f64::powf(2.78281828, -lambda)) / full_cov.len() as f64;
    let ret_ani;
    //let ani = f64::powf(1. - pi, 1./k);
    let ani = f64::powf(adj_index, 1. / k);
    if ani < 0. || ani.is_nan() {
        ret_ani = None;
    } else {
        if ani > 1. {
            ret_ani = Some(ani)
        } else {
            ret_ani = Some(ani);
        }
    }
    return ret_ani;
}

fn mle_zip(full_covs: &[u32], _k: f64) -> Option<f64> {
    let mut num_zero = 0;
    let mut count_set: HashSet<_> = HashSet::default();

    for x in full_covs {
        if *x == 0 {
            num_zero += 1;
        } else {
            count_set.insert(x);
        }
    }

    //Lack of information for inference, retun None.
    if count_set.len() == 1 {
        return None;
    }

    if full_covs.len() - num_zero < SAMPLE_SIZE_CUTOFF {
        return None;
    }

    let mean = mean(&full_covs).unwrap();
    let lambda = newton_raphson(
        (num_zero as f32 / full_covs.len() as f32).into(),
        mean.into(),
    );
    //    log::trace!("lambda,pi {} {} {}", lambda,pi, num_zero as f64 / full_covs.len() as f64);
    let ret_lambda;
    if lambda < 0. || lambda.is_nan() {
        ret_lambda = None
    } else {
        ret_lambda = Some(lambda);
    }

    return ret_lambda;
}

fn newton_raphson(rat: f64, mean: f64) -> f64 {
    let mut curr = mean / (1. - rat);
    //    dbg!(1. - mean,rat);
    for _ in 0..1000 {
        let t1 = (1. - rat) * curr;
        let t2 = mean * (1. - f64::powf(2.78281828, -curr));
        let t3 = 1. - rat;
        let t4 = mean * (f64::powf(2.78281828, -curr));
        curr = curr - (t1 - t2) / (t3 - t4);
    }
    return curr;
}

fn var(data: &[u32]) -> Option<f32> {
    if data.is_empty() {
        return None;
    }
    let data_t;
    if data.len() > VAR_CUTOFF{
        data_t = &data[0..data.len() * 95 / 100];
    }
    else{
        data_t = &data;
    }
    let mean = mean(data_t).unwrap();
    let mut var = 0.;
    for x in data_t {
        var += (*x as f32 - mean) * (*x as f32 - mean)
    }
    return Some(var / data_t.len() as f32);
}

fn mean(data: &[u32]) -> Option<f32> {
    let sum = data.iter().sum::<u32>() as f32;
    let count = data.len();

    match count {
        positive if positive > 0 => Some(sum / count as f32),
        _ => None,
    }
}

fn bootstrap_interval(
    covs_full: &Vec<u32>,
    k: f64,
    args: &ContainArgs,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    fastrand::seed(7);
    let num_samp = covs_full.len();
    let iters = 100;
    let mut res_ani = vec![];
    let mut res_lambda = vec![];

    for _ in 0..iters {
        let mut rand_vec = vec![];
        rand_vec.reserve(num_samp);
        for _ in 0..num_samp {
            rand_vec.push(covs_full[fastrand::usize(..covs_full.len())]);
        }
        let lambda;
        if args.ratio {
            lambda = ratio_lambda(&rand_vec, args.min_count_correct);
        } else if args.mme {
            lambda = mme_lambda(&rand_vec);
        } else if args.nb {
            lambda = binary_search_lambda(&rand_vec);
        } else if args.mle {
            lambda = mle_zip(&rand_vec, k);
        } else {
            lambda = ratio_lambda(&rand_vec,args.min_count_correct);
        }
        let ani = ani_from_lambda(lambda, mean(&rand_vec).unwrap().into(), k, &rand_vec);
        if ani.is_some() && lambda.is_some() {
            if !ani.unwrap().is_nan() && !lambda.unwrap().is_nan() {
                res_ani.push(ani);
                res_lambda.push(lambda);
            }
        }
    }
    res_ani.sort_by(|x, y| x.partial_cmp(y).unwrap());
    res_lambda.sort_by(|x, y| x.partial_cmp(y).unwrap());
    if res_ani.len() < 50 {
        return (None, None, None, None);
    }
    let suc = res_ani.len();
    let low_ani = res_ani[suc * 5 / 100 - 1];
    let high_ani = res_ani[suc * 95 / 100 - 1];
    let low_lambda = res_lambda[suc * 5 / 100 - 1];
    let high_lambda = res_lambda[suc * 95 / 100 - 1];

    return (low_ani, high_ani, low_lambda, high_lambda);
}

fn ratio_lambda(full_covs: &Vec<u32>, min_count_correct: f64) -> Option<f64> {
    let mut num_zero = 0;
    let mut count_map: FxHashMap<_, _> = FxHashMap::default();

    for x in full_covs {
        if *x == 0 {
            num_zero += 1;
        } else {
            let c = count_map.entry(*x as usize).or_insert(0);
            *c += 1;
        }
    }

    //Lack of information for inference, retun None.
    if count_map.len() == 1 {
        return None;
    }

    if full_covs.len() - num_zero < SAMPLE_SIZE_CUTOFF {
        return None;
    } else {
        let mut sort_vec: Vec<(_, _)> = count_map.iter().map(|x| (x.1, x.0)).collect();
        sort_vec.sort_by(|x, y| y.cmp(&x));
        let most_ind = sort_vec[0].1;
        if !count_map.contains_key(&(most_ind + 1)) {
            return None;
        }
        let count_p1 = count_map[&(most_ind + 1)] as f64;
        let count = count_map[&most_ind] as f64;
        if count_p1 < min_count_correct || count < min_count_correct{
            return None;
        }
        let lambda = Some(count_p1 / count * ((most_ind + 1) as f64));
        return lambda;
    }
}

fn mme_lambda(full_covs: &[u32]) -> Option<f64> {
    let mut num_zero = 0;
    let mut count_set: HashSet<_> = HashSet::default();

    for x in full_covs {
        if *x == 0 {
            num_zero += 1;
        } else {
            count_set.insert(x);
        }
    }

    //Lack of information for inference, retun None.
    if count_set.len() == 1 {
        return None;
    }

    if full_covs.len() - num_zero < SAMPLE_SIZE_CUTOFF {
        return None;
    }

    let mean = mean(&full_covs).unwrap();
    let var = var(&full_covs).unwrap();
    let lambda = var / mean + mean - 1.;
    if lambda < 0. {
        return None;
    } else {
        return Some(lambda as f64);
    }
}

fn _get_kmer_identity(seq_sketch: &SequencesSketch, estimate_unknown: bool) -> Option<f64>{
    if !estimate_unknown{
        return None
    }

    let mut num_1s = 0;
    let mut num_not1s = 0;
    for count in seq_sketch.kmer_counts.values(){
        if *count == 1{
            num_1s += 1;
        }
        else{
            num_not1s += *count;
        }
    }
    let eps = num_not1s as f64 / (num_not1s as f64 + num_1s as f64);
    if eps < 1.{
        return Some(eps)
    }
    else{
        return None
    }
}
