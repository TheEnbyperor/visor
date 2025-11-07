fn main() {
    println!("cargo:rustc-link-arg-bins=-Tlinkall.x");
    patch_crate::run().expect("Failed while patching");

    let out_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut eyes = vec![];
    for eye in std::fs::read_dir("./eyes").unwrap() {
        let eye = eye.unwrap();
        let eye_txt = std::fs::read(eye.path()).unwrap();
        let eye_lines = std::str::from_utf8(&eye_txt).unwrap().split('\n');
        let mut line_data = vec![];
        for line in eye_lines {
            let mut line_val = 0;
            for (i, pixel) in line.chars().enumerate() {
                if pixel == '#' {
                    line_val |= 1 << (7 - i);
                } else if pixel != '_' {
                    panic!("Unexpected pixel: {}", pixel);
                }
            }
            line_data.push(line_val);
        }
        let eye_data = line_data.into_iter().map(|i| i.to_string()).collect::<Vec<_>>().join(", ");
        eyes.push(format!("const EYE_{}: EyeImage = EyeImage([{}]);", eye.file_name().to_str().unwrap().to_uppercase(), eye_data));
    }
    let out_data = eyes.join("\n").into_bytes();
    std::fs::write(out_dir.join("eye_image_data.rs"), out_data).unwrap();
}