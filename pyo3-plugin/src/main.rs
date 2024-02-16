use nih_plug::nih_export_standalone;
use pyo3::buffer::Element;
use pyo3::types::{IntoPyDict, PyList};
use pyo3::{IntoPy, Py, PyErr, PyObject, PyTryFrom, Python};
use pyo3_plugin::PyO3Plugin;
use std::error::Error;
use std::time::Instant;

fn main() {
    let elapsed = Instant::now();
    //    Python::with_gil(|py| {
    //        let sys = py.import("sys")?;
    //        let version: String = sys.getattr("version")?.extract()?;
    //
    //        let locals = [("os", py.import("os")?)].into_py_dict(py);
    //        let code =
    //            "(os.getenv('USER') or os.getenv('USERNAME') or 'Unknown') + str(os.environ.keys())";
    //        let user: String = py.eval(code, None, Some(&locals))?.extract()?;
    //        let locals = [("np", py.import("numpy")?)].into_py_dict(py);
    //        let code = "str(np.arange(10))";
    //        let tmp: String = py.eval(code, None, Some(&locals))?.extract()?;
    //
    //        //        println!("Hello {}, I'm Python {}", user, version);
    //        //   println!("tmp: {}", tmp);
    //        Ok::<(), std::io::Error>(())
    //    })
    //    .unwrap();
    let mut data: Vec<_> = (0..1024).map(|x| x as f32).collect::<Vec<_>>();

    //let list = <PyList as PyTryFrom>::try_from(array.as_ref(py)).unwrap();
    let elapsed = Instant::now();
    let n = 1000;
    for i in 0..n {
        let new_data = Python::with_gil(|py| -> std::result::Result<Vec<f32>, PyErr> {
            let locals = [("data", data)].into_py_dict(py);
            let code = "list(map(lambda x: x + 1, data))";
            let tmp: Vec<f32> = py.eval(code, None, Some(&locals))?.extract()?;
            Ok(tmp)
        })
        .unwrap();
        data = new_data;
    }
    let t = elapsed.elapsed();
    eprintln!("{:?}", data);
    eprintln!("{}ms", t.as_millis());
    eprintln!("{}ms/iter", 1000.0 * (t.as_secs_f64() / n as f64))
    //    nih_export_standalone::<PyO3Plugin>();
}
