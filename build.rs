extern crate shaderc;

use color_eyre::Result;
use color_eyre::eyre::OptionExt;
use color_eyre::eyre::eyre;
use naga::{
    back::spv, front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator}
};
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=shaders/*");

    compile_shaders()?;

    Ok(())
}

enum ShaderLanguage {
    Glsl,
    Wgsl,
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

        let shader_lang = match ext {
            "vert" => ShaderLanguage::Glsl,
            "frag" => ShaderLanguage::Glsl,
            "comp" => ShaderLanguage::Glsl,
            "wgsl" => ShaderLanguage::Wgsl,
            _ => return Err(eyre!("Shader language not recognized for file: {:?}", path)),
        };

        let spv_binary = match shader_lang {
            ShaderLanguage::Glsl => compile_glsl(&path)?,
            ShaderLanguage::Wgsl => compile_wgsl(&path)?,
        };
        
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

fn compile_glsl(filepath: &Path) -> Result<Vec<u32>> {
    Err(eyre!("GLSL not supported yet"))
    /*
    let shader_stage = match ext {
        "vert" => ShaderStage::Vertex,
        "frag" => ShaderStage::Fragment,
        "comp" => ShaderStage::Compute,
        _ => {
            log::warn!("Skipping non-GLSL file: {:?}", path);
            return;
        }
    };
    */
}

fn compile_wgsl(filepath: &Path) -> Result<Vec<u32>> {
    // Read the WGSL file and parse into IR
    let source = fs::read_to_string(&filepath)?;
    let module = wgsl::parse_str(&source)?;
    
    // Validate the IR
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let validation_info = validator.validate(&module)?;
    log::info!("{:?}", validation_info);

    // Generate the SPIR-V binary
    Ok(spv::write_vec(&module, &validation_info, &spv::Options::default(), None)?)
}
