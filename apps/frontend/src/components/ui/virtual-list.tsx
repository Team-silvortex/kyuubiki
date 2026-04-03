"use client";

import { useState, type ReactNode, type UIEvent } from "react";

type VirtualListProps<T> = {
  items: T[];
  itemHeight: number;
  maxHeight: number;
  overscan?: number;
  className?: string;
  emptyState?: ReactNode;
  itemKey: (item: T, index: number) => string;
  renderItem: (item: T, index: number) => ReactNode;
};

export function VirtualList<T>({
  items,
  itemHeight,
  maxHeight,
  overscan = 4,
  className,
  emptyState,
  itemKey,
  renderItem,
}: VirtualListProps<T>) {
  const [scrollTop, setScrollTop] = useState(0);

  if (items.length === 0) {
    return emptyState ? <div className={className}>{emptyState}</div> : null;
  }

  const viewportHeight = Math.min(maxHeight, items.length * itemHeight);
  const totalHeight = items.length * itemHeight;
  const startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  const visibleCount = Math.ceil(viewportHeight / itemHeight) + overscan * 2;
  const endIndex = Math.min(items.length, startIndex + visibleCount);
  const offsetTop = startIndex * itemHeight;
  const visibleItems = items.slice(startIndex, endIndex);

  const handleScroll = (event: UIEvent<HTMLDivElement>) => {
    setScrollTop(event.currentTarget.scrollTop);
  };

  return (
    <div className={className}>
      <div className="virtual-list__viewport" style={{ maxHeight: `${maxHeight}px`, height: `${viewportHeight}px` }} onScroll={handleScroll}>
        <div className="virtual-list__spacer" style={{ height: `${totalHeight}px` }}>
          <div className="virtual-list__content" style={{ transform: `translateY(${offsetTop}px)` }}>
            {visibleItems.map((item, index) => (
              <div key={itemKey(item, startIndex + index)} className="virtual-list__row" style={{ minHeight: `${itemHeight}px` }}>
                {renderItem(item, startIndex + index)}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
