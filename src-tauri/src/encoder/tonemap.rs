/// Legacy stub kept for backward compatibility until probe.rs is rewritten (task 12.3).
#[allow(dead_code)]
pub fn infer_hdr_from_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("hdr") || lower.ends_with(".mov")
}

/// Determine whether the source video is HDR based on ffprobe color metadata.
///
/// Returns `true` when color_transfer is "smpte2084" (PQ) or "arib-std-b67" (HLG),
/// or color_primaries is "bt2020".
pub fn is_hdr(color_transfer: Option<&str>, color_primaries: Option<&str>) -> bool {
    if let Some(ct) = color_transfer {
        if ct == "smpte2084" || ct == "arib-std-b67" {
            return true;
        }
    }
    if let Some(cp) = color_primaries {
        if cp == "bt2020" {
            return true;
        }
    }
    false
}

/// Build the HDR → SDR tonemap filter chain for FFmpeg.
///
/// Returns `Some(filter_string)` when the source is HDR, `None` for SDR.
pub fn build_tonemap_filter(hdr: bool) -> Option<String> {
    if !hdr {
        return None;
    }
    Some(
        "zscale=t=linear:npl=100,\
         format=gbrpf32le,\
         zscale=p=bt709,\
         tonemap=hable:desat=0,\
         zscale=t=bt709:m=bt709:r=tv,\
         format=yuv420p"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdr_pq_transfer() {
        assert!(is_hdr(Some("smpte2084"), None));
    }

    #[test]
    fn hdr_hlg_transfer() {
        assert!(is_hdr(Some("arib-std-b67"), None));
    }

    #[test]
    fn hdr_bt2020_primaries() {
        assert!(is_hdr(None, Some("bt2020")));
    }

    #[test]
    fn sdr_bt709() {
        assert!(!is_hdr(Some("bt709"), Some("bt709")));
    }

    #[test]
    fn sdr_none_values() {
        assert!(!is_hdr(None, None));
    }

    #[test]
    fn tonemap_filter_hdr() {
        let filter = build_tonemap_filter(true).unwrap();
        assert!(filter.contains("zscale"));
        assert!(filter.contains("tonemap=hable"));
        assert!(filter.contains("format=yuv420p"));
    }

    #[test]
    fn tonemap_filter_sdr() {
        assert!(build_tonemap_filter(false).is_none());
    }
}
