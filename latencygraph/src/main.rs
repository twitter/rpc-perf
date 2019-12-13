
use latencygraph::*;

fn main() {
    let mut args = std::env::args().skip(1);

    let mut file = std::fs::File::open(args.next().unwrap()).unwrap();
    let sketch1 = serde_json::from_reader(&mut file).unwrap();

    let mut file = std::fs::File::open(args.next().unwrap()).unwrap();
    let sketch2 = serde_json::from_reader(&mut file).unwrap();

    plot_linear(&sketch1, &sketch2).unwrap();
    plot_log(&sketch1, &sketch2).unwrap();
}

