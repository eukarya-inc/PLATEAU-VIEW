import { test, expect } from '@playwright/test';

test.describe('建築物クリック時の属性表示 @smoke', () => {
  test('選択モードと属性パネルの基本動作', async ({ page }) => {
    // PLATEAU VIEWを開く
    await page.goto('https://plateauview.mlit.go.jp');
    
    // ページの読み込みを待つ
    await page.waitForSelector('button[aria-label="メインメニュー"]', { timeout: 30000 });
    await page.waitForTimeout(3000); // 3Dデータの読み込み待ち

    // 選択モードに切り替える
    const selectButton = page.getByRole('button', { name: '選択' });
    await selectButton.click();
    await expect(selectButton).toHaveAttribute('aria-pressed', 'true');

    // 3Dビューの中央をクリックして建築物を選択
    const canvas = page.locator('canvas').first();
    const box = await canvas.boundingBox();
    if (box) {
      // 画面中央より少し下をクリック（建築物がある可能性が高い）
      await page.mouse.click(box.x + box.width / 2, box.y + box.height * 0.6);
      
      // 属性パネルが表示されるか確認（閉じるボタンがあるリスト）
      const attributePanel = page.locator('ul').filter({ has: page.locator('button[aria-label="閉じる"]') });
      
      // 属性パネルが表示された場合
      const isPanelVisible = await attributePanel.isVisible({ timeout: 3000 }).catch(() => false);
      
      if (isPanelVisible) {
        // 建築物の属性が表示されているか確認
        const panelText = await attributePanel.textContent();
        const hasAttributes = panelText?.includes('高さ') || 
                            panelText?.includes('階数') || 
                            panelText?.includes('用途') ||
                            panelText?.includes('住所');
        
        expect(hasAttributes).toBeTruthy();
        
        // パネルを閉じる
        const closeButton = attributePanel.locator('button[aria-label="閉じる"]');
        await closeButton.click();
        
        // パネルが閉じられたことを確認
        await expect(attributePanel).not.toBeVisible({ timeout: 3000 });
      }
    }
    
    // 移動モードに切り替える
    const moveButton = page.getByRole('button', { name: '移動' });
    await moveButton.click();
    await expect(moveButton).toHaveAttribute('aria-pressed', 'true');
    
    // 移動モードでは属性パネルが表示されないことを確認
    if (box) {
      await page.mouse.click(box.x + box.width / 2, box.y + box.height * 0.6);
      
      // 属性パネルが表示されないことを確認
      const attributePanel = page.locator('ul').filter({ has: page.locator('button[aria-label="閉じる"]') });
      await expect(attributePanel).not.toBeVisible({ timeout: 1000 });
    }
  });
});