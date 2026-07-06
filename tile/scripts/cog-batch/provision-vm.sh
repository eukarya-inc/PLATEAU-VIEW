#!/usr/bin/env bash
# GCE Spot VM を立てて COG 変換を回す（使い捨て・単発）。既存 GCP スタック前提。
# R2 は egress/ingress 無料 → 読みは無料。⚠️ GCE→R2 の書き戻しは GCP egress 課金。
#
# ★ preemption: この単発VMは自動再起動しない。回収されたら手動で作り直し→同じ
#   convert-cogs.sh を再実行すれば冪等に続きから再開する（README「Spot preemption 耐性」）。
#   無人運用は MIG(Spot+autoheal) か Cloud Batch(task自動リトライ) を使う。
set -euo pipefail

PROJECT="${PROJECT:-reearth-plateau}"
ZONE="${ZONE:-asia-northeast1-b}"
NAME="${NAME:-cog-batch}"
MACHINE="${MACHINE:-c3d-highcpu-90}"     # 多コア。小さく試すなら c3d-highcpu-16
DISK_GB="${DISK_GB:-1000}"               # スクラッチ（balanced PD）。ortho流すなら≥1TB

gcloud compute instances create "$NAME" \
  --project="$PROJECT" --zone="$ZONE" \
  --machine-type="$MACHINE" \
  --provisioning-model=SPOT \
  --instance-termination-action=DELETE \
  --image-family=ubuntu-2404-lts-amd64 --image-project=ubuntu-os-cloud \
  --boot-disk-size=50GB \
  --create-disk=name="${NAME}-scratch",size="${DISK_GB}GB",type=pd-balanced,auto-delete=yes \
  --metadata=startup-script='#!/bin/bash
set -e
# scratch mount
DEV=$(lsblk -dpno NAME,SIZE | grep -v $(findmnt -no SOURCE / | sed "s/[0-9]*$//") | sort -k2 -h | tail -1 | awk "{print \$1}")
mkfs.ext4 -F "$DEV" && mkdir -p /scratch && mount "$DEV" /scratch && chmod 777 /scratch
# tools
apt-get update && apt-get install -y gdal-bin unzip curl
curl https://rclone.org/install.sh | bash
echo "provisioned: gdal $(gdalinfo --version), rclone $(rclone version | head -1)" > /var/log/cog-provision.log'

cat <<EOF

VM 作成中: $NAME ($MACHINE, SPOT, ${DISK_GB}GB scratch @ $ZONE)

次の手順:
  1. gcloud compute scp tile/scripts/cog-batch/convert-cogs.sh rclone.conf $NAME:~ --zone=$ZONE
  2. gcloud compute ssh $NAME --zone=$ZONE
  3. VM上で:
       export RCLONE_CONFIG=~/rclone.conf
       sudo WORK=/scratch J=\$(nproc) ORTHO_J=12 RC="rclone --config ~/rclone.conf --transfers=32 --checkers=64 --s3-no-check-bucket" \\
         bash convert-cogs.sh all        # or: base dem5 / patch noto / ortho 2024
  4. 終わったら: gcloud compute instances delete $NAME --zone=$ZONE
EOF
