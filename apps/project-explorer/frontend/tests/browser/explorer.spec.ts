import { test, expect } from '@playwright/test';

test('gallery filters, native links and browser history preserve URLs', async ({ page, context }) => {
  // Arrange
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('#/crates?category=Observability&q=activity');
  // Act / Assert
  await expect(page.locator('.tile')).toHaveCount(1);
  await expect(page.getByRole('heading', { name: 'Activity Tracing', exact: true })).toBeVisible();
  const href = await page.locator('.tile').getAttribute('href');
  const tab = await context.newPage();
  await tab.goto(`http://127.0.0.1:4177/doramcap/${href}`);
  await expect(tab.locator('.detail-heading h1')).toHaveText('Activity Tracing');
  await tab.close();
  // Act
  await page.locator('.tile').click();
  await page.reload();
  // Assert
  await expect(page.locator('.detail-heading h1')).toHaveText('Activity Tracing');
  await expect(page.locator('.direct-dependencies')).toBeVisible();
  // Act
  await page.goBack();
  // Assert
  await expect(page.getByRole('searchbox')).toHaveValue('activity');
  await expect(page.getByRole('combobox', { name: 'Category', exact: true })).toHaveValue('Observability');
  await expect(page.locator('.tile')).toHaveCount(1);
  expect(errors).toEqual([]);
});

test('detail renders repository SVGs and follows documentation links and anchors', async ({ page }) => {
  // Arrange
  await page.goto('#/apps/provenance-demo');
  // Act / Assert
  await expect(page.locator('.detail-heading h1')).toHaveText('Provenance Demo');
  const image = page.locator('.markdown img').first();
  await expect(image).toBeVisible();
  await expect.poll(() => image.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBeGreaterThan(0);
  await expect(image).toHaveAttribute('src', /content\/apps\/provenance-demo\/docs\/.+\.svg/);
  // Act
  await page.getByRole('link', { name: 'Implementation plan and evidence' }).click();
  // Assert
  await expect(page).toHaveURL(/#\/docs\?/);
  await expect(page.locator('.markdown h1')).toBeVisible();
  // Act
  await page.goto('#/crates/activity-tracing?heading=shutdown');
  // Assert
  await expect(page.locator('#shutdown')).toBeInViewport();
});

test('component sidebar stays direct and Explore opens the dependency DAG', async ({ page }) => {
  // Arrange
  await page.goto('#/apps/provenance-demo');
  // Assert
  await expect(page.locator('.direct-deps > li')).toHaveCount(2);
  await expect(page.getByText('Direct workspace dependencies.', { exact: true })).toHaveCount(0);
  // Act
  await page.locator('.dependency-panel').getByRole('link', { name: 'Explore', exact: true }).click();
  await page.reload();
  // Assert
  await expect(page).toHaveURL(/#\/dependencies\?focus=apps%2Fprovenance-demo/);
  await expect(page.locator('.react-flow__node')).toHaveCount(3);
  await expect(page.locator('.react-flow__controls')).toBeVisible();
  await expect(page.locator('.react-flow__handle')).not.toHaveCount(0);
  // Act
  await page.locator('.react-flow__node[data-id="step-provenance"]').click();
  // Assert
  await page.getByRole('link', { name: 'Open crate →', exact: true }).click();
  await expect(page).toHaveURL(/#\/crates\/step-provenance$/);
});

test('DAG preserves shared edges and can expand to the workspace', async ({ page }) => {
  // Arrange
  await page.goto('#/dependencies?focus=apps%2Fcuda-shm-demo');
  // Assert
  await expect(page.locator('.react-flow__node')).toHaveCount(4);
  await expect(page.locator('.react-flow__edge')).toHaveCount(4);
  await expect(page.locator('.react-flow__node[data-id="cuda-host-memory"]')).toHaveCount(1);
  // Act
  await page.getByRole('combobox', { name: 'Dependency component' }).selectOption('');
  // Assert
  await expect.poll(() => page.locator('.react-flow__node').count()).toBeGreaterThan(4);
  // Act
  await page.getByRole('checkbox', { name: 'Build', exact: true }).check();
  await page.reload();
  // Assert
  await expect(page.getByRole('checkbox', { name: 'Build', exact: true })).toBeChecked();
  await expect(page.locator('.react-flow')).toBeVisible();
});

test('mobile galleries and detail pages fit without horizontal overflow', async ({ page }) => {
  // Arrange
  await page.setViewportSize({ width: 375, height: 812 });
  // Act
  await page.goto('#/apps');
  // Assert
  await expect(page.locator('.tile').first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  // Act
  await page.goto('#/crates/storage-persistence');
  // Assert
  await expect(page.locator('.detail-heading h1')).toHaveText('Storage Persistence');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

test('unknown paths are recoverable and frontend-only apps have no invented dependencies', async ({ page }) => {
  // Arrange
  await page.goto('#/missing');
  // Assert
  await expect(page.getByRole('heading', { name: 'Page not found' })).toBeVisible();
  // Act
  await page.goto('#/apps/project-explorer');
  // Assert
  await expect(page.getByText('No workspace dependencies.', { exact: false })).toBeVisible();
});


test('overview is the home page and use-case mappings survive refresh and history', async ({ page }) => {
  // Arrange
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('');
  // Assert
  await expect(page.getByRole('heading', { name: 'The doramcap foundation' })).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Use cases' }).getByRole('link')).toHaveCount(3);
  await expect(page.getByRole('link', { name: 'Run a perception pipeline', exact: true })).toHaveAttribute('aria-current', 'page');
  await expect(page.locator('.mapping-detail')).toContainText('CUDA SHM Demo');
  await expect(page.locator('.workload-labels svg')).toHaveCount(4);
  await expect(page.locator('.platform-stack .workload-layer')).toBeVisible();
  await expect(page.locator('.platform-stack .foundation-layer')).toBeVisible();
  await expect(page.locator('.platform-stack .platform-layer').filter({ has: page.getByRole('heading', { name: 'Compute', exact: true }) })).toBeVisible();
  await expect(page.locator('.scenario-tabs svg, .scenario-tabs small')).toHaveCount(0);
  await expect(page.locator('.software-layer')).toContainText('Linux');
  await expect(page.locator('.software-layer')).toContainText('CUDA');
  expect(await page.locator('.mapping-detail').evaluate(el => Boolean(el.compareDocumentPosition(document.querySelector('.platform-stack')!) & Node.DOCUMENT_POSITION_FOLLOWING))).toBe(true);
  // Act
  await page.getByRole('link', { name: 'Run a perception pipeline', exact: true }).click();
  await page.reload();
  // Assert
  await expect(page).toHaveURL(/useCase=perception/);
  await expect(page.locator('.mapping-detail')).toContainText('CUDA SHM Demo');
  // Act
  await page.getByRole('link', { name: 'Dora IPC', exact: true }).click();
  // Assert
  await expect(page.locator('.mapping-detail')).toContainText('Node communication');
  await expect(page.getByRole('link', { name: 'Upstream project' })).toHaveAttribute('href', 'https://dora-rs.ai/');
  // Act
  await page.goBack();
  // Assert
  await expect(page.locator('.mapping-detail')).toContainText('Run a perception pipeline');
  await expect(page.getByText('Illustrative use case', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Your runtime foundation for Physical AI.');
  await expect(page.getByRole('heading', { name: 'Explore use cases', exact: true })).toBeVisible();
  await page.locator('.mapping-links a').filter({ hasText: 'Provenance Demo' }).click();
  await expect(page).toHaveURL(/#\/apps\/provenance-demo$/);
  expect(errors).toEqual([]);
});

test('overview capability deep links and mobile layout work', async ({ page }) => {
  // Arrange
  await page.setViewportSize({ width: 320, height: 812 });
  await page.goto('#/overview?capability=recording');
  // Assert
  await expect(page.locator('.mapping-detail')).toContainText('Storage Persistence');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  // Act
  await page.goto('#/overview?useCase=unknown');
  // Assert
  await expect(page.getByRole('alert')).toContainText('not in the current overview');
});


test('the locally served manifest and schema are available for audit', async ({ page, request }) => {
  // Arrange
  await page.goto('#/overview');
  // Act
  const response = await request.get('content/apps/project-explorer/overview.json');
  const schema = await request.get('content/apps/project-explorer/overview.schema.json');
  // Assert
  expect(response.ok()).toBe(true);
  expect((await response.json()).useCases.map((u: { id: string }) => u.id)).toContain('perception');
  expect(schema.ok()).toBe(true);
  expect((await schema.json()).additionalProperties).toBe(false);
});


test('execution substrate links retain the use case and explain platform scope', async ({ page }) => {
  // Arrange
  await page.goto('#/overview?useCase=perception');
  // Act
  await page.locator('.software-layer').getByRole('link', { name: 'CUDA platform-dependent' }).click();
  await page.reload();
  // Assert
  await expect(page).toHaveURL(/useCase=perception&integration=cuda/);
  await expect(page.locator('.mapping-detail')).toContainText('Platform-dependent');
  await expect(page.locator('.mapping-detail')).toContainText('CUDA Host Memory');
  // Act
  await page.locator('.software-layer').getByRole('link', { name: 'Linux', exact: true }).click();
  // Assert
  await expect(page.locator('.mapping-detail')).toContainText('Processes, scheduling, memory');
});


test('focused crate sits between its dependents and dependencies', async ({ page }) => {
  // Arrange
  await page.goto('#/dependencies?focus=crates%2Fcuda-dora-shm');
  // Act
  await page.locator('.react-flow').waitFor();
  const selected = page.locator('.react-flow__node[data-id="cuda-dora-shm"]');
  const above = await page.locator('.react-flow__node[data-id="cuda-shm-demo"]').boundingBox();
  const middle = await selected.boundingBox();
  const below = await page.locator('.react-flow__node[data-id="cuda-host-memory"]').boundingBox();
  // Assert
  await expect(selected).toHaveClass(/focus-node/);
  expect(above!.y + above!.height).toBeLessThan(middle!.y);
  expect(middle!.y + middle!.height).toBeLessThan(below!.y);
});
