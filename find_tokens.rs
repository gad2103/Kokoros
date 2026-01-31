use kokoros::tts::tokenize::tokenize;

fn main() {
    let text = "heɪ ðɪs ɪz ˈlʌvliː!";
    let tokens = tokenize(text);
    println!("{:?}", tokens);
}
