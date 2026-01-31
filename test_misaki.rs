use misaki_rs::g2p::en::G2P;

fn main() {
    let g2p = G2P::new(false); // trf = false (FAST PATH)

    let text = "Hello world, this is a phonemizer test.";
    let phonemes = g2p.convert(text);

    println!("{:?}", phonemes);
}
