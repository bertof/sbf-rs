use std::error::Error;

use rayon::prelude::*;

use crate::data_structure::SBF;
use crate::types::HashFunction;

#[test]
fn test_sbf() -> Result<(), Box<dyn Error>> {
    let mut sbf = SBF::new(10 as u8, 2, 5,
                           HashFunction::MD5, 3)?;
    #[cfg(feature = "serde_support")] {
        println!("{}", serde_json::to_string(&sbf)?);
    }
    assert!(sbf.filter.par_iter().all(|v| *v == 0));

    sbf.insert(b"test".to_vec(), 1).expect("Correct insertion of an area");
    #[cfg(feature = "serde_support")] {
        println!("{}", serde_json::to_string(&sbf)?);
    }
    let count = sbf.filter.par_iter().cloned().filter(|v| *v == 1).count();
    assert!(2 >= count && count > 0);
    let filter = sbf.filter.clone();

    sbf.insert(b"test".to_vec(), 1).expect("Correct insertion of an area");
    #[cfg(feature = "serde_support")] {
        println!("{}", serde_json::to_string(&sbf)?);
    }
    assert_eq!(filter, sbf.filter);

    sbf.insert(b"test1".to_vec(), 2).expect("Correct insertion of an area");
    #[cfg(feature = "serde_support")] {
        println!("{}", serde_json::to_string(&sbf)?);
    }
    let filter = sbf.filter.clone();

    sbf.insert(b"test1".to_vec(), 2).expect("Correct insertion of an area");
    #[cfg(feature = "serde_support")] {
        println!("{}", serde_json::to_string(&sbf)?);
    }
    assert_eq!(filter, sbf.filter);

    #[cfg(feature = "metrics")] {
        sbf.metrics.set_area_fpp();
        sbf.metrics.set_prior_area_fpp();
        sbf.metrics.set_area_isep();
        sbf.metrics.set_prior_area_isep();
        #[cfg(feature = "serde_support")] {
            println!("{}", serde_json::to_string(&sbf)?);
        }
        println!("AREA MEMBERS: {:?}", sbf.metrics.get_area_members(1));
        assert_eq!(2, sbf.metrics.get_area_members(1).unwrap());

        println!("FILTER SPARSITY: {}", sbf.metrics.get_filter_sparsity());
        println!("FILTER FPP: {}", sbf.metrics.get_filter_fpp());
        println!("EXPECTED AREA EMERSION 1: {}", sbf.metrics.get_expected_area_emersion(1));
        println!("AREA EMERSION 1: {}", sbf.metrics.get_area_emersion(1).unwrap_or(-1.0));
        println!("FILTER PRIOR FPP: {}", sbf.metrics.get_filter_prior_fpp());
    }

    Ok(())
}