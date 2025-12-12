use cargo_metadata::{Metadata, MetadataCommand};
use anyhow::{Context, bail, Result};
use std::env;
use std::path::PathBuf;
use std::fs::{create_dir_all, write};
use crate::helper::Helper;


pub struct WindowsBuilder {
    release: bool,
    target: String,
    cwd: PathBuf,
    output_path: Option<PathBuf>
}

impl WindowsBuilder {
    pub fn start(release: bool, target: String, cwd: PathBuf) {
    println!("Building for Windows");
    let mut op = WindowsBuilder::new(release, target, cwd);
    //TODO check for signing certificate
    op.pre_build();

    //>>build
    op.build();

    //>>Postbuild
    //TODO move binary to the app bundle and sign
    op.post_build();
    }

    fn new(release: bool, target: String, cwd: PathBuf) -> Self {
        println!("creating windowsBuilder: release: {:?}, target: {:?}, cwd: {:?}", release, target.to_string(), cwd);
        WindowsBuilder{release: release, target: target.to_string(), cwd: cwd, output_path: None}
    }

    fn pre_build(&mut self) -> Result<()>{
        //parse cargo.toml
        let metadata: Metadata = match MetadataCommand::new()
            .current_dir(self.cwd.clone())
            .exec() {
                Ok(md) => md,
                Err(_) => bail!("error parsing cargo toml")
            };

        // Read standard fields from the first package
        if let Some(package) = metadata.packages.first() {
            println!("Package name: {}", package.name);
            println!("Version: {}", package.version);
            // //Example Read dependencies (example: check if "clap" is a dep)
            // if let Some(dep) = package.dependencies.iter().find(|d| d.name == "clap") {
            //     println!("Clap dependency version req: {}", dep.req);
            // }
            // Read custom [package.metadata] keys (if present)
            if let serde_json::Value::Object(meta) = &package.metadata {
                //parse icon_path from the cargo.toml
                if let Some(icon_path) = meta.get("icon_path") {
                    println!("icon_path: {}", icon_path);
                    self.configure_bundle(Some(icon_path.to_string()));
                }else {
                    println!("no icon path found");
                    self.configure_bundle(None);
                }
            }
        } else {
            println!("No packages found in Cargo.toml");
        }
        Ok(())
    }

    fn configure_bundle(&mut self, icon_path: Option<String>) -> Result <()>{
        println!("building the dynamic app bundle");
        let cwd: PathBuf = self.cwd.clone();
        println!("working dir: {:?}", cwd);
        let rel_output: PathBuf = if self.release {
            "target/release/windows".into()
        }else {"target/debug/windows".into()};
        self.output_path = Some(cwd.join(&rel_output));
        println!("windows dir: {:?}", self.output_path);
        //empty the target directory if it exists
        let path = self.output_path.as_ref().unwrap().as_path();
        if path.exists() && path.is_dir(){
            Helper::empty_directory(path)?
        }
        //create the target directory
        create_dir_all(self.output_path.as_ref().unwrap())?;
        let rc_path: PathBuf = self.output_path.as_ref().unwrap().join("app.rc");
        let rc_icon: &PathBuf = &rel_output.join("windows_icon.ico");
        let content = format!("IDI_ICON1 ICON \"{}\"", rc_icon.display());
        //create the app.rc file
        write(&rc_path, content.as_bytes())?;
        println!("created {:?} with content: {}", rc_path, content);   
        //TODO open question: Will the App.rc compiling break the bundle if the user does not provide an icon?
        println!("Icon path: {}", &icon_path.clone().unwrap());
        //if no icon was provided
        if icon_path.is_none() {
            return Ok(())
        //if icon was provided, format the app icon for windows
        }else {
        //TODO convert the .png at the icon_path to a .ico which resides in the app bundle
        }
        println!("done configuring Windows bundle");
        Ok(())
        
    }

    fn build(&self) -> Result<()> {
        println!("building");
        //TODO build the app binary
        Ok(())
    }

    fn post_build(&self) -> Result<()>{
        println!("post building");
        //TODO move the app binary to the proper location
        Ok(())
    }
}




// pub fn convert_png_to_ico(session: &Session, input_path: &str) -> io::Result<()> {
//     let windows = "windows_icon.ico";
//     let win_output_path = format!(
//         "{}/{}/assets/resources/icons/{}",
//         session.projects_path.as_ref().unwrap(),
//         session.current_project.as_ref().unwrap(),
//         windows
//     );
//     // Open the PNG file
//     let img = image::open(input_path).map_err(|e| {
//         io::Error::new(
//             io::ErrorKind::Other,
//             format!("Failed to open {}: {}", input_path, e),
//         )
//     })?;

//     // Resize to the specified size
//     let resized = imageops::resize(&img, 64, 64, imageops::FilterType::Lanczos3);
//     let resized_img = DynamicImage::ImageRgba8(resized);

//     // Write as ICO
//     let file = std::fs::File::create(win_output_path.clone())?;
//     let mut writer = std::io::BufWriter::new(file);
//     let encoder = image::codecs::ico::IcoEncoder::new(&mut writer);
//     encoder
//         .write_image(
//             resized_img.as_bytes(),
//             64,
//             64,
//             image::ExtendedColorType::Rgba8,
//         )
//         .map_err(|e| {
//             io::Error::new(
//                 io::ErrorKind::Other,
//                 format!("Failed to save {} as ICO: {}", win_output_path, e),
//             )
//         })?;
//     println!(
//         "Converted {} to ICO ({}x{}) and saved as {}",
//         input_path, 64, 64, win_output_path
//     );

//     //check for app.rc and if it exists remove it
//     let rc = format!(
//         "{}/{}/app.rc",
//         session.projects_path.as_ref().unwrap(),
//         session.current_project.as_ref().unwrap()
//     );
//     if Path::new(&rc).exists() {
//         let output = Command::new("rm").arg(&rc).output().unwrap();
//         if !output.status.success() {
//             return Err(io::Error::new(
//                 io::ErrorKind::Other,
//                 "could not remove old app.rc: {}",
//             ));
//         }
//     }
//     //create a new app.rc using absolute path passed in
//     let ico_path = format!(
//         "{}/{}/assets/resources/icons/windows_icon.ico",
//         session.projects_path.as_ref().unwrap(),
//         session.current_project.as_ref().unwrap()
//     );
//     let rc_content = format!(r#"IDI_ICON1 ICON "{}""#, ico_path);
//     let mut rc_file = File::create(&rc)?;
//     rc_file.write_all(rc_content.as_bytes())?;
//     //ensure the file is fully written
//     rc_file.flush()?;
//     //explicitly close the file
//     drop(rc_file);
//     println!("created resource file: {}", &rc);
//     let res = format!(
//         "{}/{}/app.res",
//         session.projects_path.as_ref().unwrap(),
//         session.current_project.as_ref().unwrap()
//     );
//     println!("rc path: {}", &rc);
//     println!("res path: {}", &res);
//     let build_path = format!(
//         "{}/{}/build.rs",
//         session.projects_path.as_ref().unwrap(),
//         session.current_project.as_ref().unwrap()
//     );
//     //if a build.rs file exists, first remove it.
//     if Path::new(&build_path).exists() {
//         let output = Command::new("rm")
//             .arg(&build_path)
//             .output()
//             .unwrap();
//         if !output.status.success() {
//             return Err(io::Error::new(
//                 io::ErrorKind::Other,
//                 "could not remove old build.rs",
//             ));
//         }
//     }
//     //populate the build.rs content
//     let build_content = format!(
//         r#"
//         use std::io;

//         fn main() {{
//             if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" && std::path::Path::new("{}").exists() {{
//                 embed_resource::compile("app.rc", embed_resource::NONE)
//                 .manifest_optional();
//             }}
//     }}
    
//         "#,
//         &ico_path
//     );
//     //Generate a build.rs file
//     let mut build_file = fs::File::create(&build_path)?;
//     build_file.write_all(build_content.as_bytes())?;
//     build_file.flush()?;
//     println!("Created Build.rs at {}", &build_path);
//     //copy windows_icon.ico into a favicon.ico
//     let output = Command::new("cp")
//         .args([&win_output_path, &wasm_output_path])
//         .output()
//         .unwrap();

//     if !output.status.success() {
//         return Err(io::Error::new(
//             io::ErrorKind::Other,
//             "could not copy favicon: {}",
//         ));
//     }
//     println!(
//         "copied {} ({}x{}) as {}",
//         win_output_path, 64, 64, wasm_output_path
//     );
//     Ok(())
// }






