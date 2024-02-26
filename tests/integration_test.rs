use assert_cmd::prelude::*; // Add methods on commands
use std::str;
use std::fs;
use std::path::Path;
use serial_test::serial;
use std::process::Command; // Run programs

fn fresh(){
    Command::new("rm")
        .arg("-r")
        .args(["./tests/results/test_sketch_dir"])
        .spawn();
}

#[serial]
#[test]
fn test_basic(){
    let mut output = Command::cargo_bin("fairy").unwrap();
    let output = output
        .arg("coverage")
        .arg("./test_files/o157_reads_100.fastq.gz")
        .arg("./test_files/e.coli-o157.fasta.gz")
        .output()
        .expect("Output failed");
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");
    dbg!(stdout.matches('\n').count());
    assert!(stdout.matches('\n').count() == 3);
}

#[serial]
#[test]
fn test_sketch(){
    fresh();
    let mut cmd = Command::cargo_bin("fairy").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("./test_files/coli1.fq.gz")
        .arg("-2")
        .arg("./test_files/coli2.fq.gz")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/coli1.fq.gz.paired.bcsp").exists(), "Output file was not created");

    let mut cmd = Command::cargo_bin("fairy").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("./test_files/coli1.fq.gz")
        .arg("-2")
        .arg("./test_files/coli2.fq.gz")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--lS")
        .arg("./test_files/sample_list.txt")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/S1.paired.bcsp").exists(), "Output file was not created");

    let mut cmd = Command::cargo_bin("fairy").unwrap();
    let output = cmd
        .arg("coverage")
        .arg("./tests/results/test_sketch_dir/S1.paired.bcsp")
        .arg("./test_files/e.coli-o157.fasta.gz")
        .output()
        .expect("Output failed");
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");

    //stdout is a tsv file. I want to check if the 2nd row's 3rd column is > 0
    let mut lines = stdout.lines();
    lines.next();
    let line = lines.next().unwrap();
    let mut cols = line.split('\t');
    cols.next();
    cols.next();
    let cov1 = cols.next().unwrap().parse::<f64>().unwrap();
    let cov2 = cols.next().unwrap().parse::<f64>().unwrap();
    dbg!("{},{}",cov1, cov2);
    assert!(cov1 == cov2);
    assert!(cov2 > 0.1);

    let mut cmd = Command::cargo_bin("fairy").unwrap();
    let output = cmd
        .arg("coverage")
        .arg("./tests/results/test_sketch_dir/coli1.fq.gz.paired.bcsp")
        .arg("./test_files/e.coli-o157.fasta.gz")
        .output()
        .expect("Output failed");
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");

    //stdout is a tsv file. I want to check if the 2nd row's 3rd column is > 0
    let mut lines = stdout.lines();
    lines.next();
    let line = lines.next().unwrap();
    let mut cols = line.split('\t');
    cols.next();
    cols.next();
    let cov1 = cols.next().unwrap().parse::<f64>().unwrap();
    let cov2 = cols.next().unwrap().parse::<f64>().unwrap();
    dbg!("{},{}",cov1, cov2);
    assert!(cov1 == cov2);
    assert!(cov2 > 0.1);
}

fn test_profile_vs_query(){

    let mut output = Command::cargo_bin("sylph").unwrap();
    let output = output
        .arg("profile")
        .arg("./test_files/o157_reads.fastq")
        .arg("./test_files/e.coli-EC590.fasta")
        .output()
        .expect("Output failed");
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");
    dbg!(stdout.matches('\n').count());
    assert!(stdout.matches('\n').count() == 2);

    let mut output = Command::cargo_bin("sylph").unwrap();
    let output = output
        .arg("query")
        .arg("./test_files/o157_reads.fastq")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("./test_files/e.coli-o157.fasta")
        .arg("./test_files/e.coli-K12.fasta")
        .output()
        .expect("Output failed");
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");
    dbg!(stdout.matches('\n').count());
    println!("{}",stdout);
    assert!(stdout.matches('\n').count() == 4);
}

#[serial]
fn test_sketch_commands() {
    Command::new("rm")
        .arg("-r")
        .args(["./tests/results/test_sketch_dir"])
        .spawn();
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("./test_files/e.coli-K12.fasta")
        .arg("./test_files/o157_reads.fastq")
        .arg("./test_files/e.coli-W.fasta.gz")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("profile")
        .arg("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp")
        .arg("./tests/results/test_sketch_dir/db.syldb")
        .assert();
    assert.success().code(0);

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("profile")
        .arg("-l")
        .arg("./test_files/list.txt")
        .assert();
    assert.success().code(0);


    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("profile")
        .arg("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp")
        .arg("./test_files/e.coli-EC590.fasta")
        .assert();
    assert.success().code(0);

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("profile")
        .arg("./test_files/o157_reads.fastq")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("-i")
        .arg("-m")
        .arg("90")
        .assert();
    assert.success().code(0);

    let mut cmd= Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("./test_files/t1.fq")
        .arg("-2")
        .arg("./test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/t1.fq.paired.sylsp").exists(), "Output file was not created");
    fresh();

    let mut cmd= Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("--l1")
        .arg("./test_files/pair_list1.txt")
        .arg("--l2")
        .arg("./test_files/pair_list2.txt")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/t1.fq.paired.sylsp").exists(), "Output file was not created");



    fresh();
    let mut cmd= Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-g")
        .arg("./test_files/t1.fq")
        .arg("-r")
        .arg("./test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/testdb")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/t2.fq.sylsp").exists(), "Output file was not created");
    assert!(Path::new("./tests/results/test_sketch_dir/testdb.syldb").exists(), "Output file was not created");

    fresh();
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-r")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("./test_files/o157_reads.fastq")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/e.coli-EC590.fasta.sylsp").exists(), "Output file was not created");
    assert!(Path::new("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp").exists(), "Output file was not created");
    assert!(!Path::new("./tests/results/test_sketch_dir/db.syldb").exists(), "Output file was created");
    fresh();

    fresh();
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-g")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("./test_files/o157_reads.fastq")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(!Path::new("./tests/results/test_sketch_dir/e.coli-EC590.fasta.sylsp").exists(), "Output file was created");
    assert!(!Path::new("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp").exists(), "Output file was created");
    assert!(Path::new("./tests/results/test_sketch_dir/db.syldb").exists(), "Output file was not created");
    fresh();

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("--gl")
        .arg("test_files/list.txt")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/db.syldb").exists(), "Output file was not created");
    fresh();

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("--rl")
        .arg("test_files/list.txt")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);
    assert!(!Path::new("./tests/results/test_sketch_dir/db.syldb").exists(), "Output file was not created");
    assert!(Path::new("./tests/results/test_sketch_dir/e.coli-EC590.fasta.sylsp").exists(), "Output file was not created");
    assert!(Path::new("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp").exists(), "Output file was not created");
    fresh();

}

#[serial]
fn test_profile_disabling(){
    fresh();

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-g")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--disable-profiling")
        .assert();
    assert.success().code(0);

    let mut output = Command::cargo_bin("sylph").unwrap();
    let assert = output
        .arg("profile")
        .arg("./test_files/o157_reads.fastq")
        .arg("./tests/results/test_sketch_dir/db.syldb")
        .assert();
    assert.failure().code(1);

    let mut output = Command::cargo_bin("sylph").unwrap();
    let assert = output
        .arg("query")
        .arg("./test_files/o157_reads.fastq")
        .arg("./tests/results/test_sketch_dir/db.syldb")
        .assert();
    assert.success().code(0);

    fresh();
}

#[serial]
fn test_sketch_fasta_fastq_concord(){
    fresh();
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("./test_files/e.coli-EC590.fasta")
        .arg("./test_files/o157_reads.fastq")
        .arg("-o")
        .arg("./tests/results/test_sketch_dir/db")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .assert();
    assert.success().code(0);

    let mut output = Command::cargo_bin("sylph").unwrap();
    let out1 = output
        .arg("profile")
        .arg("./test_files/o157_reads.fastq")
        .arg("./tests/results/test_sketch_dir/db.syldb")
        .output()
        .expect("Fail");

    let mut output = Command::cargo_bin("sylph").unwrap();
    let out2 = output
        .arg("profile")
        .arg("./test_files/o157_reads.fastq")
        .arg("./test_files/e.coli-EC590.fasta")
        .output()
        .expect("Fail");

    let mut output = Command::cargo_bin("sylph").unwrap();
    let out3 = output
        .arg("profile")
        .arg("./tests/results/test_sketch_dir/o157_reads.fastq.sylsp")
        .arg("./tests/results/test_sketch_dir/db.syldb")
        .output()
        .expect("Fail");

    let stdout1 = str::from_utf8(&out1.stdout).expect("Output was not valid UTF-8");
    let stdout2 = str::from_utf8(&out2.stdout).expect("Output was not valid UTF-8");
    let stdout3 = str::from_utf8(&out3.stdout).expect("Output was not valid UTF-8");

    assert!(stdout1 == stdout2);
    assert!(stdout1 == stdout3);
    assert!(stdout2 == stdout3);

    fresh();
}

#[serial]
fn test_sample_names(){
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("test_files/t1.fq")
        .arg("-2")
        .arg("test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--lS")
        .arg("./test_files/single_sample.txt")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/SAMPLE_TEST.paired.sylsp").exists(), "Output file was not created");
    fresh();

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("test_files/t1.fq")
        .arg("test_files/o157_reads.fastq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--lS")
        .arg("./test_files/sample_list.txt")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/S1.sylsp").exists(), "Output file was not created");
    assert!(Path::new("./tests/results/test_sketch_dir/S2.sylsp").exists(), "Output file was not created");

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let output = cmd
        .arg("profile")
        .arg("./tests/results/test_sketch_dir/S2.sylsp")
        .arg("./test_files/e.coli-EC590.fasta")
        .output().unwrap();
    let stdout = str::from_utf8(&output.stdout).expect("Output was not valid UTF-8");
    dbg!(&stdout);
    assert!(stdout.contains("S2"));
    assert!(!stdout.contains("o157_reads"));

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("test_files/t1.fq")
        .arg("-2")
        .arg("test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("-S")
        .arg("SAMPLE_TEST_S")
        .assert();
    assert.success().code(0);
    assert!(Path::new("./tests/results/test_sketch_dir/SAMPLE_TEST_S.paired.sylsp").exists(), "Output file was not created, -S");
    fresh();


}

#[serial]
fn test_fpr(){
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("test_files/t1.fq")
        .arg("-2")
        .arg("test_files/t2.fq")
        .arg("-d ")
        .arg("./tests/results/test_sketch_dir")
        .arg("0")
        .assert();
    assert.success().code(0);
    fresh();

    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("test_files/t1.fq")
        .arg("-2")
        .arg("test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--fpr")
        .arg("0.001")
        .assert();
    assert.success().code(0);
    fresh();
    let mut cmd = Command::cargo_bin("sylph").unwrap();
    let assert = cmd
        .arg("sketch")
        .arg("-1")
        .arg("test_files/t1.fq")
        .arg("-2")
        .arg("test_files/t2.fq")
        .arg("-d")
        .arg("./tests/results/test_sketch_dir")
        .arg("--fpr")
        .arg("2")
        .assert();
    assert.failure().code(1);
    fresh();

}
