use color_eyre::Result;
use color_eyre::eyre::OptionExt;
use naga::{
    back::spv, front::glsl::{Frontend, Options},
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage
};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=shaders/*");

    compile_shaders()?;

    Ok(())
}

fn compile_shaders() -> Result<()> {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let shaders_in_dir = Path::new(&cargo_manifest_dir).join("shaders");
    let shaders_out_dir = Path::new(&cargo_manifest_dir).join("shaders-built");

    for entry in fs::read_dir(shaders_in_dir)? {
        let entry = entry?;
        let path = entry.path();

        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_eyre(format!("Shader file has no extension: {:?}", path))?;
        let shader_stage = match ext {
            "vert" => ShaderStage::Vertex,
            "frag" => ShaderStage::Fragment,
            "comp" => ShaderStage::Compute,
            _ => {
                log::warn!("Skipping non-GLSL file: {:?}", path);
                continue;
            }
        };

        // Read the GLSL file and parse into IR
        let source = fs::read_to_string(&path)?;
        let mut frontend = Frontend::default();
        let module = frontend.parse(&Options::from(shader_stage), &source)?;
        
        // Validate the IR
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        let validation_info = validator.validate(&module)?;
        log::info!("{:?}", validation_info);

        // Generate the SPIR-V binary
        let spv_binary = spv::write_vec(&module, &validation_info, &spv::Options::default(), None)?;

        // Write the SPIR-V binary to a file
        let shader_name = path
            .file_stem()
            .ok_or_eyre("Shader file has no name")?
            .to_str()
            .ok_or_eyre("Shader file name is not valid UTF-8")?;
        let output_filepath = shaders_out_dir
            .join(format!("{}.spv", shader_name));
        fs::create_dir_all(output_filepath.parent().ok_or_eyre("No parent")?)?;
        fs::write(output_filepath, bytemuck::cast_slice(&spv_binary))?;
    }

    Ok(())
}
