pub use bcevm::primitives::kzg::{parse_kzg_trusted_setup, G1Points, G2Points, KzgErrors};
use std::{env, fs, path::PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cmd {
    #[structopt(required = true)]
    path: PathBuf,
    #[structopt(long)]
    g1: Option<PathBuf>,
    #[structopt(long)]
    g2: Option<PathBuf>,
}

impl Cmd {
    pub fn run(&self) -> Result<(), KzgErrors> {
        if !self.path.exists() {
            return Err(KzgErrors::PathNotExists);
        }
        let out_dir = env::current_dir().map_err(|_| KzgErrors::FailedCurrentDirectory)?;
        let kzg_trusted_settings = fs::read_to_string(&self.path).map_err(|_| KzgErrors::NotValidFile)?;
        let (g1, g2) = parse_kzg_trusted_setup(&kzg_trusted_settings)?;
        let g1_path = self.g1.clone().unwrap_or_else(|| out_dir.join("g1_points.bin"));
        let g2_path = self.g2.clone().unwrap_or_else(|| out_dir.join("g2_points.bin"));
        
        fs::write(&g1_path, flatten(&g1.0)).map_err(|_| KzgErrors::IOError)?;
        fs::write(&g2_path, flatten(&g2.0)).map_err(|_| KzgErrors::IOError)?;
        
        println!("Finished formatting kzg trusted setup into binary representation.");
        println!("G1 points path: {:?}", g1_path);
        println!("G2 points path: {:?}", g2_path);
        Ok(())
    }
}

fn flatten<const N: usize, const M: usize>(x: &[[u8; N]; M]) -> &[u8] {
    unsafe { core::slice::from_raw_parts(x.as_ptr().cast(), N * M) }
}
