
pub struct AndroidBuilder {
    release: bool,
    target: String,
}

impl AndroidBuilder {
    pub fn start() -> Self{
    println!("building for android");
    //>>prebuild
    //-check for signing certificate
    //setup the app bundle

    //>>build

    //>>Postbuild
    //move binary to the app bundle and sign
    AndroidBuilder{release: false, target: "target".to_string()}
    }
}

struct AndroidRunner{
device: String, 
}

impl AndroidRunner{
    fn new() -> Self{
        println!("running for android");

        AndroidRunner{device: "device".to_string()}
    }
}