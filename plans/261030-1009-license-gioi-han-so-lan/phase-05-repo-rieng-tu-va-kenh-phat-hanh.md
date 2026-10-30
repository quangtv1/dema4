---
phase: 5
title: "Repo rieng tu va kenh phat hanh"
status: pending
priority: P2
dependencies: []
effort: "0.5 ngày"
---

# Phase 5: Repo riêng tư và kênh phát hành công khai

## Overview
Chuyển `quangtv1/dema4` sang riêng tư; file exe phát hành ở repo công khai `quangtv1/dema4-releases` (chỉ chứa README + Releases). CI của repo riêng tư đẩy file sang; nút Kiểm tra cập nhật và link tải trỏ sang repo mới.

## Requirements
- Người dùng vẫn tải được bản mới và app vẫn tự báo có bản mới.
- Không lộ mã nguồn qua repo phát hành.

## Architecture
- Repo `quangtv1/dema4-releases` (công khai): README ngắn (giới thiệu, ảnh, cách mở file chưa ký, liên hệ Sao Mai).
- Secret `RELEASES_TOKEN` (fine-grained PAT, quyền Contents: write chỉ trên repo phát hành) ở repo riêng tư.
- `build.yml` bước "Tạo/cập nhật Release": `softprops/action-gh-release@v2` thêm `repository: quangtv1/dema4-releases`, `token: ${{ secrets.RELEASES_TOKEN }}`, `tag_name: ${{ github.ref_name }}`.
- `tro_giup.rs`: `API_PHAT_HANH` / `TRANG_TAI` → `quangtv1/dema4-releases`.

## Related Code Files
- Modify: `.github/workflows/build.yml`, `dema4_rs/src/tro_giup.rs`, `README.md` (link tải)
- Create (repo khác): `quangtv1/dema4-releases/README.md`

## Tests trước
1. `link_phat_hanh_tro_repo_cong_khai`: `API_PHAT_HANH` và `TRANG_TAI` chứa `dema4-releases`.
2. Test sẵn có `doc_tag_tu_json_phat_hanh` vẫn xanh (định dạng API không đổi).

## Implementation Steps
1. Viết test 1, đỏ → sửa hằng → xanh.
2. Tạo repo `dema4-releases` + README (`gh repo create quangtv1/dema4-releases --public`).
3. Tạo PAT fine-grained, thêm secret `RELEASES_TOKEN` (`gh secret set`).
4. Sửa `build.yml`; đẩy tag thử `v0.6.0-rc1` → Release xuất hiện ở repo phát hành, xoá bản thử sau.
5. **Người dùng xác nhận rồi mới** `gh repo edit quangtv1/dema4 --visibility private --accept-visibility-change-consequences`.
6. Chép các Release cũ (v0.5.x) sang repo phát hành nếu muốn giữ lịch sử tải.

## Success Criteria
- [ ] Tag mới tạo Release ở `dema4-releases` với 2 file exe.
- [ ] App bản mới báo đúng phiên bản mới nhất từ repo phát hành.
- [ ] Repo `dema4` riêng tư; trang `dema4-releases` không có mã nguồn.

## Risk Assessment
- Phút GitHub Actions riêng tư: 2.000 phút/tháng, macOS ×10 → ~8 lần phát hành/tháng; giảm bằng chỉ build khi đẩy tag (bỏ build mỗi lần push main) nếu cần.
- Bản cũ (≤ v0.5.x) đang hỏi API repo cũ: khi repo riêng tư, API trả 404 → nút Kiểm tra cập nhật báo lỗi; người dùng cũ cần tải tay bản mới một lần (ghi trong thông báo phát hành).
- PAT hết hạn → CI không đẩy được Release; đặt hạn dài và ghi nhớ gia hạn.
