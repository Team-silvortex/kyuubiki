"use client";

import { useLayoutEffect, useRef, type HTMLAttributes, type ReactNode } from "react";

type WorkbenchPanelPagesProps = HTMLAttributes<HTMLDivElement> & {
  navigation: ReactNode;
  pageKey: string;
  contentClassName?: string;
};

export function WorkbenchPanelPages({
  navigation, pageKey, children, className = "", contentClassName = "sidebar-stack", ...props
}: WorkbenchPanelPagesProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    // A new page must not inherit the previous page's position in a long form.
    if (contentRef.current) contentRef.current.scrollTop = 0;
  }, [pageKey]);

  return (
    <div {...props} className={`workbench-panel-pages ${className}`}>
      <div className="workbench-panel-navigation" data-workbench-panel-navigation="true">
        {navigation}
      </div>
      <div ref={contentRef} className={`${contentClassName} panel-scroll-window workbench-panel-content`}
        data-workbench-panel-content={pageKey} tabIndex={0}>
        {children}
      </div>
    </div>
  );
}
