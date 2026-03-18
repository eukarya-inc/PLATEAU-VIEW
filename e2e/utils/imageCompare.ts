import { Page } from '@playwright/test';
import { PNG } from 'pngjs';
import fs from 'fs';
import path from 'path';

/**
 * 画像の黒成分の割合を計算
 * @param imagePath - 画像ファイルのパス
 * @returns 黒色ピクセルの割合（0-1）
 */
export async function calculateBlackPixelRatio(imagePath: string): Promise<number> {
  return new Promise((resolve, reject) => {
    try {
      fs.createReadStream(imagePath)
        .pipe(new PNG())
        .on('parsed', function() {
          const totalPixels = this.width * this.height;
          let blackPixels = 0;

          // 各ピクセルをチェック（RGBA形式）
          for (let y = 0; y < this.height; y++) {
            for (let x = 0; x < this.width; x++) {
              const idx = (this.width * y + x) << 2;
              const r = this.data[idx];
              const g = this.data[idx + 1];
              const b = this.data[idx + 2];
              
              // 黒色の判定（RGB値がすべて低い）
              // しきい値を50に設定（完全な黒以外も含む）
              if (r < 50 && g < 50 && b < 50) {
                blackPixels++;
              }
            }
          }

          resolve(blackPixels / totalPixels);
        })
        .on('error', (error) => {
          console.error(`Error processing image ${imagePath}:`, error);
          reject(error);
        });
    } catch (error) {
      console.error(`Error reading image ${imagePath}:`, error);
      reject(error);
    }
  });
}

/**
 * 地図領域のスクリーンショットを撮影
 * @param page - Playwright Page インスタンス
 * @param filename - 保存するファイル名
 * @returns スクリーンショットのパス
 */
export async function captureMapArea(page: Page, filename: string): Promise<string> {
  // スクリーンショット保存ディレクトリ
  const screenshotDir = path.join(process.cwd(), 'test-screenshots');
  if (!fs.existsSync(screenshotDir)) {
    fs.mkdirSync(screenshotDir, { recursive: true });
  }
  
  const filepath = path.join(screenshotDir, filename);
  
  // Cesiumキャンバスエリアのスクリーンショットを撮影
  const canvas = page.locator('canvas').first();
  await canvas.screenshot({ path: filepath });
  
  return filepath;
}

/**
 * 2つの画像の黒成分比率を比較
 * @param beforePath - 変更前の画像パス
 * @param afterPath - 変更後の画像パス
 * @returns 黒成分が増加したかどうか
 */
export async function compareBlackRatio(
  beforePath: string,
  afterPath: string,
  threshold: number = 0.1
): Promise<{
  increased: boolean;
  beforeRatio: number;
  afterRatio: number;
  difference: number;
}> {
  const beforeRatio = await calculateBlackPixelRatio(beforePath);
  const afterRatio = await calculateBlackPixelRatio(afterPath);
  const difference = afterRatio - beforeRatio;
  
  return {
    increased: difference > threshold,
    beforeRatio,
    afterRatio,
    difference
  };
}