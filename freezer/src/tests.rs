use crate::freezer_files::helper::truncate_file;
use crate::freezer_files::{FreezerFilesBuilder, INDEX_ENTRY_SIZE};

fn make_bytes(size: usize, byte: u8) -> Vec<u8> {
    let mut ret = Vec::with_capacity(size);
    ret.resize_with(size, || byte);
    ret
}

#[test]
fn basic() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();

    for i in 1..100 {
        let data = make_bytes(15, i);
        freezer.append(i.into(), &data).unwrap();
    }

    for i in 1..50 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }

    for i in 100..255 {
        let data = make_bytes(15, i);
        freezer.append(i.into(), &data).unwrap();
    }

    for i in 1..255 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }
}

#[test]
fn reopen() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();

    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .max_file_size(50)
            .build()
            .unwrap();

        freezer.preopen().unwrap();
        for i in 1..255 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }
    }

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();

    for i in 1..255 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }
}

#[test]
fn try_repair_dangling_head1() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();

    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .max_file_size(50)
            .build()
            .unwrap();

        freezer.preopen().unwrap();
        for i in 1..255 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }

        let metadata = freezer.index.metadata().unwrap();
        truncate_file(&mut freezer.index, metadata.len() - 4).unwrap();
    }

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();

    assert_eq!(freezer.retrieve(0xfd).unwrap(), Some(make_bytes(15, 0xfd)));
    assert_eq!(freezer.retrieve(0xff).unwrap(), None);
}

#[test]
fn try_repair_dangling_head2() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();

    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .max_file_size(50)
            .build()
            .unwrap();

        freezer.preopen().unwrap();
        for i in 1..255 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }

        truncate_file(
            &mut freezer.index,
            INDEX_ENTRY_SIZE * 2 + INDEX_ENTRY_SIZE / 2,
        )
        .unwrap();
    }
    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .max_file_size(50)
            .build()
            .unwrap();
        freezer.preopen().unwrap();
        assert_eq!(freezer.retrieve(1).unwrap(), Some(make_bytes(15, 1)));
        assert_eq!(freezer.retrieve(2).unwrap(), None);

        // should be able to append from 2
        for i in 2..255 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }
    }

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();

    for i in 1..255 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }
}

#[test]
fn try_repair_dangling_index() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();

    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .enable_compression(false)
            .max_file_size(50)
            .build()
            .unwrap();

        freezer.preopen().unwrap();
        for i in 1..10 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }

        for i in 1..10 {
            let expect = make_bytes(15, i);
            let actual = freezer.retrieve(i.into()).unwrap();
            assert_eq!(Some(expect), actual);
        }

        truncate_file(&mut freezer.head.file, 20).unwrap();
    }

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .enable_compression(false)
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();

    assert_eq!(freezer.number(), 8);
    assert_eq!(freezer.head.file.metadata().unwrap().len(), 15);
    for i in 1..8 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }
}

#[test]
fn truncate() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();
    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .enable_compression(false)
            .max_file_size(50)
            .build()
            .unwrap();

        freezer.preopen().unwrap();
        for i in 1..30 {
            let data = make_bytes(15, i);
            freezer.append(i.into(), &data).unwrap();
        }

        for i in 1..30 {
            let expect = make_bytes(15, i);
            let actual = freezer.retrieve(i.into()).unwrap();
            assert_eq!(Some(expect), actual);
        }

        let retrieve_out_of_bound = freezer.retrieve(30).unwrap();
        assert_eq!(None, retrieve_out_of_bound);
    }

    {
        let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
            .enable_compression(false)
            .max_file_size(50)
            .build()
            .unwrap();
        freezer.preopen().unwrap();
        freezer.truncate(10).unwrap();

        for i in 1..11 {
            let expect = make_bytes(15, i);
            let actual = freezer.retrieve(i.into()).unwrap();
            assert_eq!(Some(expect), actual);
        }

        let retrieve_out_of_bound = freezer.retrieve(11).unwrap();
        assert_eq!(None, retrieve_out_of_bound);
        assert_eq!(freezer.number(), 11);
        assert_eq!(freezer.head.bytes, 15);
    }

    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .enable_compression(false)
        .max_file_size(50)
        .build()
        .unwrap();
    freezer.preopen().unwrap();
    freezer.truncate(1).unwrap();

    let expect = make_bytes(15, 1);
    let actual = freezer.retrieve(1).unwrap();
    assert_eq!(Some(expect), actual);

    let retrieve_out_of_bound = freezer.retrieve(2).unwrap();
    assert_eq!(None, retrieve_out_of_bound);
}

#[test]
fn open_files_limit_retrieve() {
    let tempdir = tempfile::Builder::new().tempdir().unwrap();
    let mut freezer = FreezerFilesBuilder::new(tempdir.path().to_path_buf())
        .enable_compression(false)
        .max_file_size(10)
        .open_files_limit(2)
        .build()
        .unwrap();

    freezer.preopen().unwrap();
    for i in 1..100 {
        let data = make_bytes(15, i);
        freezer.append(i.into(), &data).unwrap();
    }

    assert_eq!(freezer.number(), 100);
    for i in 1..100 {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }

    for i in (1..100).rev() {
        let expect = make_bytes(15, i);
        let actual = freezer.retrieve(i.into()).unwrap();
        assert_eq!(Some(expect), actual);
    }
}
