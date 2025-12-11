
pub struct LinuxBuilder {
    release: bool,
    target: String,
}

impl LinuxBuilder {
    pub fn start() -> Self {
    println!("building for linux");
    //>>prebuild
    //-check for signing certificate?
    //setup the app bundle

    //>>build

    //>>Postbuild
    //move binary to the app bundle and sign
    LinuxBuilder{release: false, target: "target".to_string()}
    }
}

struct LinuxRunner{
device: String, 
}

impl LinuxRunner{
    fn new() -> Self {
        println!("Running for Linux");
        LinuxRunner{device: "device".to_string()}
    }
}