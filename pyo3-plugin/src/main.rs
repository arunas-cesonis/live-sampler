use nih_plug::nih_export_standalone;
use pyo3::buffer::Element;
use pyo3::types::{IntoPyDict, PyByteArray, PyList};
use pyo3::{IntoPy, Py, PyErr, PyObject, PyTryFrom, Python};
use pyo3_plugin::PyO3Plugin;
use std::error::Error;
use std::io::BufWriter;
use std::time::Instant;

fn main() {
    //    let elapsed = Instant::now();
    //    //    Python::with_gil(|py| {
    //    //        let sys = py.import("sys")?;
    //    //        let version: String = sys.getattr("version")?.extract()?;
    //    //
    //    //        let locals = [("os", py.import("os")?)].into_py_dict(py);
    //    //        let code =
    //    //            "(os.getenv('USER') or os.getenv('USERNAME') or 'Unknown') + str(os.environ.keys())";
    //    //        let user: String = py.eval(code, None, Some(&locals))?.extract()?;
    //    //        let locals = [("np", py.import("numpy")?)].into_py_dict(py);
    //    //        let code = "str(np.arange(10))";
    //    //        let tmp: String = py.eval(code, None, Some(&locals))?.extract()?;
    //    //
    //    //        //        println!("Hello {}, I'm Python {}", user, version);
    //    //        //   println!("tmp: {}", tmp);
    //    //        Ok::<(), std::io::Error>(())
    //    //    })
    //    //    .unwrap();
    //    let mut data: Vec<_> = (0..1024).map(|x| x as f32).collect::<Vec<_>>();
    //
    //    //let list = <PyList as PyTryFrom>::try_from(array.as_ref(py)).unwrap();
    //    let elapsed = Instant::now();
    //    let n = 1000;
    //    let gil = Python::acquire_gil();
    //    let py = gil.python();
    //    let mut ret: Vec<u8> = Vec::new();
    //    py.run(
    //        r"
    //import numpy as np
    //def doit2(s):
    //    return list(map(lambda x: x + 1, s))
    //def doit(s):
    //    return (np.asarray(s) + 1).tolist()
    //",
    //        None,
    //        None,
    //    );
    //    for i in 0..n {
    //        //let new_data = Python::with_gil(|py| -> std::result::Result<Vec<f32>, PyErr> {
    //        let mut buf = vec![];
    //        let mut b = BufWriter::new(&mut buf);
    //        let b: Vec<u8> = data
    //            .iter()
    //            .flat_map(|x| x.to_le_bytes().into_iter())
    //            .collect();
    //
    //        let ba: &PyByteArray = PyByteArray::new(py, &b);
    //        let locals = [("data", ba)].into_py_dict(py);
    //        let code = "(np.frombuffer(data, dtype=np.float32) + 1).tobytes()";
    //        ret = py
    //            .eval(code, None, Some(&locals))
    //            .unwrap()
    //            .extract()
    //            .unwrap();
    //        //    Ok(tmp)
    //        //})
    //        // .unwrap();
    //        //data = tmp;
    //    }
    //    let t = elapsed.elapsed();
    //    ret.
    //    eprintln!("{:?}", ret);
    //    eprintln!("{}ms", t.as_millis());
    //    eprintln!("{}ms/iter", 1000.0 * (t.as_secs_f64() / n as f64))
    nih_export_standalone::<PyO3Plugin>();
}
