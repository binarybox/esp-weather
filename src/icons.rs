pub fn convert_rgb565_to_binary(rgb565_data: &[u8]) -> Vec<u8> {
    let mut binary_data = Vec::new();

    for chunk in rgb565_data.chunks(2 * 8) {
        let mut byte = 0u8;
        for (index, pixel) in chunk.chunks(2).enumerate() {
            let pixel = ((pixel[0] as u16) << 8) | (pixel[1] as u16);
            let r = (pixel >> 11) & 0x1F;
            let g = (pixel >> 5) & 0x3F;
            let b = pixel & 0x1F;

            let luminance = (r as u32 * 30 + g as u32 * 59 + b as u32 * 11) / 100;

            if luminance > 40 {
                byte |= 1 << index;
            };
        }
        binary_data.push(byte);
    }

    binary_data.reverse(); // this must be done otherwise the images are weard

    binary_data
}
