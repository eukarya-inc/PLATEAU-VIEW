import {
  Dialog,
  dialogClasses,
  DialogContent,
  DialogTitle,
  IconButton,
  styled,
} from "@mui/material";
import { FC } from "react";

import { CloseIcon } from "../prototypes/ui-components";

type DetailsModalProps = {
  open: boolean;
  onClose?: () => void;
};

const DetailsModal: FC<DetailsModalProps> = ({ open, onClose }) => {
  return (
    <StyledDialog open={open} onClose={onClose} maxWidth="mobile">
      <StyledDialogTitle>
        <IconButton onClick={onClose}>
          <CloseIcon />
        </IconButton>
      </StyledDialogTitle>
      <StyledDialogContent>
        <h6>Data provided by:</h6>
        <ul>
          <li>
            <a href="https://maps.gsi.go.jp/development/ichiran.html">国土地理院</a>
          </li>
          <li>
            Map tiles by <a href="https://stamen.com/">Stamen Design</a>, under{" "}
            <a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>. Data by{" "}
            <a href="https://www.openstreetmap.org/">OpenStreetMap</a>, under{" "}
            <a href="https://www.openstreetmap.org/copyright">ODbL</a>.
          </li>
        </ul>
      </StyledDialogContent>
    </StyledDialog>
  );
};

export default DetailsModal;

const StyledDialog = styled(Dialog)(({ theme }) => ({
  [`.${dialogClasses.paper}`]: {
    padding: theme.spacing(1),
  },
}));

const StyledDialogTitle = styled(DialogTitle)(() => ({
  padding: 0,
  display: "flex",
  justifyContent: "flex-end",
  alignItems: "center",
}));

const StyledDialogContent = styled(DialogContent)(({ theme }) => ({
  padding: theme.spacing(1, 2),
  maxWidth: "433px",

  "& h6": {
    fontSize: theme.typography.body1.fontSize,
    marginBottom: theme.spacing(3),
  },
  "& ul": {
    listStyleType: "disc",
    paddingLeft: theme.spacing(3),
    margin: 0,
    fontSize: theme.typography.body2.fontSize,
  },
  "& li": {
    marginBottom: theme.spacing(1),
  },
  "& a": {
    color: theme.palette.text.primary,
    textDecoration: "underline",
  },
}));
