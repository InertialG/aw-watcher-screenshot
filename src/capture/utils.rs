use image::{DynamicImage, imageops};

pub fn dhash(image: &DynamicImage) -> u64 {
    let resolution = 8;
    let resized = imageops::resize(
        image,
        resolution + 1,
        resolution,
        imageops::FilterType::Nearest,
    );
    let gray = imageops::grayscale(&resized);

    let mut hash = 0u64;
    for y in 0..resolution {
        for x in 0..resolution {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left < right {
                hash |= 1 << (y * resolution + x);
            }
        }
    }
    hash
}

pub fn hamming_distance(hash1: u64, hash2: u64) -> u32 {
    (hash1 ^ hash2).count_ones()
}
