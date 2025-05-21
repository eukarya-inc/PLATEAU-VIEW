import React from "react";

export interface Props {
  className?: string;
}

const draggableViewStyle = {
  position: "absolute" as const,
  // https://github.com/reearth/reearth/blob/d8d9f34dd90abd198c924ebbb8d3553bf8d31c85/web/src/services/theme/reearthTheme/common/zIndex.ts#L16
  zIndex: 600,
  top: 0,
  left: 0,
  width: "100%",
  height: "100%",
  // https://github.com/reearth/reearth/blob/714fb912927cfa370decad90ef9429a6d306950c/web/src/services/theme/reearthTheme/common/colors.ts#L3
  background: "#3B3CD0",
  opacity: 0.5,
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
};

const dragMessageStyle = {
  // https://github.com/reearth/reearth/blob/7fff78c5c92473b30e756f33d5e6567c0f3c2545/web/src/services/theme/reearthTheme/darkTheme/index.ts#L21
  color: "rgba(0,0,0,0.2)",
  opacity: 1,
};

const DropHolder: React.FC<Props> = ({ className }) => {
  // Memo: i18n is removed here
  // since this ("Drop here") is the only place that i18n is used
  // Maybe we don't need the text here.

  return (
    <div className={className} style={draggableViewStyle}>
      <div style={dragMessageStyle}>Drop here</div>
    </div>
  );
};

export default DropHolder;
