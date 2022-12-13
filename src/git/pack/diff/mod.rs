use diffs::Diff;
#[allow(dead_code)]
#[derive(Debug,Default)]
struct DeltaDiff{
   /// keep all instruction
   ops:Vec<DeltaOp>,
   ///Structural Similarity,相似性
   ssam:usize,
   ssam_r:f64,
   ops_code:Vec<u8>,
}
#[derive(Debug,Clone, Copy)]
enum Optype {
    DATA, //插入的数据
    COPY, // 数据复制
}
#[allow(dead_code)]
#[derive(Debug,Clone, Copy)]
struct DeltaOp{
    /// instruction type
    ins:Optype, 
    /// data begin position
    begin: usize,
    /// data long 
    len:usize,
}
impl DeltaOp {
    pub fn conver_to_dalta(&self)-> Vec<u8>{
        todo!()
        // match self.ins{
        //     Optype::DATA =>{},
        //     Optype::COPY => {},
        // }
    }
    
}
impl DeltaDiff {
    fn conver_to_delta(&self)-> Vec<u8>{
        todo!();
        // let mut result  =  Vec::new();
        // for op in &self.ops {
        //     todo!()
        // }
        // vec![];
    }
}
impl Diff for DeltaDiff{
    type Error = ();

    fn equal(&mut self, _old: usize, _new: usize, _len: usize) -> Result<(), Self::Error> {
        println!("equal {:?} {:?} {:?}", _old, _new, _len);
        self.ssam+=_len;
        self.ops.push(DeltaOp{ins:Optype::COPY,begin:_new,len:_len,});
        Ok(())
    }

    fn insert(&mut self, _o: usize, _n: usize, _len: usize) -> Result<(), ()> {
        println!("insert {:?} {:?} {:?}", _o, _n, _len);
        self.ops.push(DeltaOp{ins:Optype::DATA,begin:_n,len:_len,});
        Ok(())
    }
    fn replace(&mut self, _o: usize, _l: usize, _n: usize, _nl: usize) -> Result<(), ()> {
        println!("replace {:?} {:?} {:?} {:?}", _o, _l, _n, _nl);
        Ok(())
    }
}
#[cfg(test)]
mod tests{
    use crate::git::pack::diff::DeltaDiff;

       //diff Test 
       #[test]
       fn test_imara_diff() {
        use diffs::myers;
        let a: &[usize] = &[0, 1, 3, 4, 5];
        let b: &[usize] = &[0, 1, 4, 5, 8, 9];
           
        let mut diff = DeltaDiff::default();
        myers::diff(&mut diff, a, 0, a.len(), b, 0, b.len()).unwrap();
        
        println!("{:?}",diff);
       }
   
}