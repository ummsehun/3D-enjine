# Web Import Bridge (GLB vs PMX/VMD)

## 목적

- 웹 경로에서 `GLB` 표시 품질/싱크를 우선 검증합니다.
- `PMX/VMD`는 런타임 직접 파싱이 아니라 웹 브릿지 비교 경로로만 검증합니다.

## 로더 경로 비교

- `GLTFLoader`:
  - 입력: `.glb` / `.gltf`
  - 사용 위치: `/Users/user/miku/preview-web/app.js`
  - 역할: 현재 기본 렌더 경로(운영 기준)
- `MMDLoader`:
  - 입력: `.pmx` + `.vmd`
  - 사용 위치: `/Users/user/miku/preview-web/mmd_probe.js` 기반 점검 페이지(`/mmd-probe`)
  - 역할: PMX/VMD 원본과 GLB 변환 결과의 정합성 점검(브릿지 용도)

## PMX/VMD -> GLB 변환 기준

- 스테이지/모델은 웹 및 런타임 공통 경로를 위해 GLB를 기준 포맷으로 사용합니다.
- 카메라 VMD는 좌표계/단위 보정을 포함해 적용합니다.
  - 기본 보정 인자: `camera_align_preset`, `camera_unit_scale`, `camera_vmd_fps`
- 변환 시 확인 항목:
  - 머티리얼 색공간: baseColor/emissive는 sRGB, normal/metallicRoughness/occlusion은 linear
  - 알파 모드/컷오프 일치
  - 본(스킨)/모프(표정) 채널 보존
  - 클립 길이와 오디오 길이 차이에 대한 sync profile 적용 여부

## 현재 런타임 반영 범위

- 반영됨:
  - `sync profile` 자동 로드/저장(`assets/sync/profiles.json`)
  - `/state` sync 메타 노출:
    - `sync_offset_ms`
    - `sync_profile_key`
    - `sync_profile_hit`
    - `sync_drift_ema`
    - `sync_hard_snap_count`
  - `/mmd-probe` 브릿지 검증 페이지
- 미반영:
  - Rust 런타임의 PMX/VMD 직접 렌더 파이프라인 전환

## 검증 절차(웹)

1. `cargo run -- preview --glb /path/to/model.glb --camera-vmd /path/to/camera.vmd --camera-mode blend`
2. 브라우저에서 `http://127.0.0.1:8787` 접속
3. `MMD probe` 링크(`http://127.0.0.1:8787/mmd-probe`)에서 브릿지 메타 확인
4. `/state`에서 sync/profile 값이 런타임 기대값과 일치하는지 확인
