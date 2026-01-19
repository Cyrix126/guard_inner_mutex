mod test {

    use guard_inner_mutex::InnerGuarded;
    use guard_inner_mutex_derive::InnerGuard;
    use parking_lot::Mutex;
    use std::sync::Arc;
    #[derive(InnerGuard)]
    struct UnitStructString(Arc<Mutex<String>>);

    #[derive(InnerGuard)]
    struct UnitStructNumber(Arc<Mutex<u32>>);

    #[allow(dead_code)]
    #[derive(InnerGuard)]
    struct MultipleFields {
        name: String,
        #[guard]
        inner: Arc<Mutex<Vec<u8>>>,
        count: usize,
    }
    #[test]
    fn test_single_tuple_struct() {
        let s = UnitStructString(Arc::new(Mutex::new("hello".to_string())));
        assert_eq!(*s.lock(), "hello");
        *s.lock() = "world".to_string();
        assert_eq!(*s.lock(), "world");
    }
    #[test]
    fn test_multiple_fields_struct() {
        let s = MultipleFields {
            name: "test".to_string(),
            inner: Arc::new(Mutex::new(vec![1, 2, 3])),
            count: 42,
        };
        assert_eq!(*s.lock(), vec![1, 2, 3]);
        s.lock().push(4);
        assert_eq!(*s.lock(), vec![1, 2, 3, 4]);
    }
    #[test]
    fn test_concurrent_access() {
        let s = Arc::new(UnitStructNumber(Arc::new(Mutex::new(0))));
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let s = Arc::clone(&s);
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        *s.lock() += 1;
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(*s.lock(), 1000);
    }
}
