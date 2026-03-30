import { test, expect } from '@playwright/test';

test('Milestone 1: Selection Sort Visualization', async ({ page }) => {
  await page.goto('/samantha');

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

  // Input Python code
  const editor = page.locator('.python-editor');
  await editor.fill(pythonCode);

  // Click Run
  await page.click('text=Run (Manual)');

  // Wait for AOT result - SAM States should be visible
  // For [5, 2, 4, 3, 1]:
  // n=5
  // init: 1
  // i=0: j=1,2,3,4 (4 compares), 1 swap -> 5 steps
  // i=1: j=2,3,4 (3 compares), 1 swap -> 4 steps
  // i=2: j=3,4 (2 compares), 1 swap -> 3 steps
  // i=3: j=4 (1 compare), 1 swap -> 2 steps
  // i=4: 0 compares, 1 swap -> 1 step
  // finished: 1
  // Total: 1 + 5 + 4 + 3 + 2 + 1 + 1 = 17 steps?
  // Let's just wait for it to be > 0 first.
  
  await expect(page.locator('text=SAM States: 18')).toBeVisible({ timeout: 10000 });
  
  // Verify SVG is present
  const svg = page.locator('svg');
  await expect(svg).toBeVisible();

  // Verify path elements
  const paths = svg.locator('path');
  await expect(paths).toHaveCount({ min: 1 });
});
