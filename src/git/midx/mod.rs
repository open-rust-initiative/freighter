//! TODO: midx部分的解码
//! [multi-pack-index 的使用](https://git-scm.com/docs/git-multi-pack-index)
//! [midx format](https://git-scm.com/docs/pack-format)
#[cfg(test)]
pub mod tests{
  
    fn test_asci(c:&[u8]){
        for i in c {
            print!("{:x} ",i);
        }println!();
    }

    #[test]
    fn test_all_asci(){
        test_asci(b"PAND");
        test_asci(b"OIDF");
        test_asci(b"OIDL");
        test_asci(b"OOFF");
    }
}