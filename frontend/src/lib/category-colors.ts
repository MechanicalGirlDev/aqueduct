import type { NodeCategory } from '@/stores/graphStore';

/**
 * Tailwind CSS class for category background color.
 * Maps to the CSS custom properties --cat-{name}.
 */
export const categoryBg: Record<NodeCategory, string> = {
  math: 'bg-cat-math',
  string: 'bg-cat-string',
  logic: 'bg-cat-logic',
  time: 'bg-cat-time',
  convert: 'bg-cat-convert',
  other: 'bg-cat-other',
};

/**
 * Tailwind CSS class for category text color.
 */
export const categoryText: Record<NodeCategory, string> = {
  math: 'text-cat-math',
  string: 'text-cat-string',
  logic: 'text-cat-logic',
  time: 'text-cat-time',
  convert: 'text-cat-convert',
  other: 'text-cat-other',
};

/**
 * Tailwind CSS class for category border-left color.
 */
export const categoryBorderLeft: Record<NodeCategory, string> = {
  math: 'border-l-cat-math',
  string: 'border-l-cat-string',
  logic: 'border-l-cat-logic',
  time: 'border-l-cat-time',
  convert: 'border-l-cat-convert',
  other: 'border-l-cat-other',
};

/**
 * Resolve the computed hex color for a category.
 * Used by ReactFlow MiniMap which requires a concrete color string.
 */
export function getCategoryColor(category: NodeCategory): string {
  const style = getComputedStyle(document.documentElement);
  const value = style.getPropertyValue(`--cat-${category}`).trim();
  return value !== '' ? value : '#6b7280';
}
