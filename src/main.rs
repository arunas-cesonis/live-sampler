use live_sampler::LiveSampler;
use nih_plug::nih_export_standalone;
use triple_buffer::TripleBuffer;

fn main() {
    //#[derive(Clone, Debug)]
    //struct Data {
    //    v: Vec<f32>,
    //}
    //let data = Data {
    //    v: vec![1.0, 2.0, 3.0, 4.0],
    //};

    //let (mut input, mut output) = TripleBuffer::new(&data).split();
    //input.input_buffer().v.push(1244.0);
    //input.publish();
    //eprintln!("{:?}", output.output_buffer());
    //nih_export_standalone::<LiveSampler>();
    let x = 2.6;
    eprintln!("{}", (x % 2.0));
    //nih_export_standalone::<LiveSampler>();
}
