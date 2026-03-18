import { styled } from "@mui/material";
import { useAtomValue } from "jotai";
import { FC, memo, useState } from "react";

import { readyAtom } from "../prototypes/view/states/app";
import { PLATEAUVIEW_DATA_ATTRIBUTION_DOM_ID } from "../shared/ui-components/common";

import cesiumIonCredit from "./assets/cesium-ion-credit.png";
import DetailsModal from "./DetailsModal";

export const Widget: FC = memo(function WidgetPresenter() {
  const [detailsOpen, setDetailsOpen] = useState(false);
  const ready = useAtomValue(readyAtom);

  return (
    <div id={PLATEAUVIEW_DATA_ATTRIBUTION_DOM_ID}>
      {ready && (
        <Wrapper>
          <a target="_blank" rel="noopener noreferrer" href="https://cesium.com">
            <img src={cesiumIonCredit} alt="Cesium Ion" />
          </a>
          <DetailsTrigger onClick={() => setDetailsOpen(true)}>Data Attribution</DetailsTrigger>
          <DetailsModal open={detailsOpen} onClose={() => setDetailsOpen(false)} />
        </Wrapper>
      )}
    </div>
  );
});

const Wrapper = styled("div")(({ theme }) => ({
  display: "flex",
  alignItems: "center",
  gap: theme.spacing(1.5),
  zIndex: 1,
}));

const DetailsTrigger = styled("div")(({ theme }) => ({
  cursor: "pointer",
  color: "#fff",
  fontSize: theme.typography.body2.fontSize,
  fontWeight: theme.typography.fontWeightMedium,
  "&:hover": {
    textDecoration: "none",
  },
}));
