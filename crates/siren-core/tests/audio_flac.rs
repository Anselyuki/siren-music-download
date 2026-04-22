use anyhow::Result;
use siren_core::audio::FlacMetadata;
use siren_core::{save_audio, tag_flac, OutputFormat};

fn build_test_wav() -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec).expect("wav writer");
        for sample in [0_i16, 1024, -1024, 512, -512] {
            writer.write_sample(sample).expect("sample");
        }
        writer.finalize().expect("finalize");
    }
    cursor.into_inner()
}

#[test]
fn writes_flac_vorbis_comments_after_wav_conversion() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let wav_bytes = build_test_wav();
    let flac_path = save_audio(&wav_bytes, temp_dir.path(), "test-song", OutputFormat::Flac)?;

    let artists = vec![String::from("Test Artist")];
    let album_artists = vec![String::from("Test Album Artist")];

    tag_flac(
        &flac_path,
        &FlacMetadata {
            title: "Test Song",
            artists: &artists,
            album: "Test Album",
            album_artists: &album_artists,
            track_number: Some(2),
            total_tracks: Some(9),
            disc_number: Some(1),
            total_discs: Some(1),
            cover: None,
        },
    )?;

    let tag = metaflac::Tag::read_from_path(&flac_path)?;
    let comments = tag
        .vorbis_comments()
        .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;

    assert_eq!(
        comments.title().map(|items| items.as_slice()),
        Some([String::from("Test Song")].as_slice())
    );
    assert_eq!(
        comments.artist().map(|items| items.as_slice()),
        Some([String::from("Test Artist")].as_slice())
    );
    assert_eq!(
        comments.album().map(|items| items.as_slice()),
        Some([String::from("Test Album")].as_slice())
    );
    assert_eq!(
        comments.album_artist().map(|items| items.as_slice()),
        Some([String::from("Test Album Artist")].as_slice())
    );
    assert_eq!(comments.track(), Some(2));
    assert_eq!(comments.total_tracks(), Some(9));
    assert_eq!(comments.get("DISCNUMBER"), Some(&vec![String::from("1")]));
    assert_eq!(comments.get("TOTALDISCS"), Some(&vec![String::from("1")]));

    Ok(())
}
