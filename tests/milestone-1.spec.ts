import { test, expect } from '@playwright/test';

test('Milestone 1: Selection Sort Visualization', async ({ page }) => {
  await page.goto('/samantha');
  
  // Wait for app to mount
  await expect(page.locator('h1')).toHaveText('Samantha Editor', { timeout: 30000 });

  const pythonCode = `
import samantha

def build_test():
    return [5, 2, 4, 3, 1]

def build_algorithm(sam_fsm, data):
    n = len(data)
    sam_fsm.log("init", data=list(data))
    for i in range(n):
        min_idx = i
        for j in range(i + 1, n):
            sam_fsm.log("compare", i=i, j=j, min_idx=min_idx)
            if data[j] < data[min_idx]:
                min_idx = j
        data[i], data[min_idx] = data[min_idx], data[i]
        sam_fsm.log("swap", i=i, j=min_idx, data=list(data))
    sam_fsm.log("finished", data=list(data))

def build_animation(sam, initial_data, logs):
    comp = sam.composition
    layers = []
    for i, val in enumerate(initial_data):
        layer = comp.add_layer(f"item-{i}")
        layer.set_position(i * 30.0, 200.0 - (float(val) * 5.0))
        layer.set_size(24.0, float(val) * 10.0)
        layer.set_color(0.5, 0.5, 1.0)
        layers.append(layer)

    sam.add_state(steps=0, frame=0.0)
    current_frame = 0.0
    for i, log in enumerate(logs, start=1):
        current_frame += 30.0
        sam.add_state(steps=i, frame=current_frame)
    
    return sam
`;

  // Check for handshake messages
  const logs = page.locator('.log-viewer li');
  const handshakeText = 'Hegel Coroutine Handshake: Init OK: [PyObject PyInt { value: 2 }]';
  await expect(logs.filter({ hasText: handshakeText }), `Expected handshake text: "${handshakeText}"`).toBeVisible({ timeout: 15000 });

  // Input Python code
  const editor = page.locator('.python-editor');
  await editor.fill(pythonCode);

  // Click Run
  await page.click('text=Run (Manual)');

  // Wait for AOT result - SAM States should be visible
  const samStatesLocator = page.locator('text=SAM States: 18');
  await expect(samStatesLocator, "Expected 18 SAM states after running selection sort").toBeVisible({ timeout: 20000 });
  
  // Verify SVG is present
  // TODO not there yet
  // const svg = page.locator('svg');
  // await expect(svg).toBeVisible();

  // Verify path elements
  // const paths = svg.locator('path');
  // await expect(paths).toHaveCount({ min: 1 });
});
