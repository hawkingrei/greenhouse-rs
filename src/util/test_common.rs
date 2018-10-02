use rand::{thread_rng, Rng};

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut result = vec![];
    let mut rng = thread_rng();
    for _ in 0..n {
        result.push(rng.gen_range(0, 255) & 0xFF);
    }
    result
}
