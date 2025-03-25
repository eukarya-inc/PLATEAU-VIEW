import {
  alpha,
  styled,
  Table,
  TableBody,
  TableCell,
  tableCellClasses,
  TableRow,
  tableRowClasses,
  Typography,
} from "@mui/material";
import { forwardRef } from "react";

const Root = styled("div", {
  shouldForwardProp: prop => prop !== "gutterBottom",
})<{ gutterBottom?: boolean }>(({ theme, gutterBottom = false }) => ({
  ...(gutterBottom && {
    marginBottom: theme.spacing(1),
  }),
}));

const StyledTable = styled(Table)(({ theme }) => ({
  [`& .${tableCellClasses.head}`]: {
    color: theme.palette.text.secondary,
  },
  [`& .${tableCellClasses.root}`]: {
    // Match the light style of divider.
    // https://github.com/mui/material-ui/blob/v5.13.1/packages/mui-material/src/Divider/Divider.js#L71
    borderBottomColor: alpha(theme.palette.divider, 0.08),
  },
  [`& .${tableRowClasses.root}:last-of-type .${tableCellClasses.root}`]: {
    borderBottomWidth: 0,
  },
}));

const StyledTableCell = styled(TableCell)(({ theme }) => ({
  padding: theme.spacing(0, 2),
}));

export interface PropertyActionItemProps {
  name: string;
  children?: React.ReactNode;
}

export const PropertyActionItem = forwardRef<HTMLDivElement, PropertyActionItemProps>(
  ({ name, children, ...props }, ref) => {
    return (
      <Root ref={ref} {...props}>
        <StyledTable size="small">
          <TableBody>
            <TableRow style={{ wordBreak: "break-all" }}>
              <StyledTableCell width="100%">
                <Typography variant={"body2"}>{name}</Typography>
              </StyledTableCell>
              <StyledTableCell align="right">{children}</StyledTableCell>
            </TableRow>
          </TableBody>
        </StyledTable>
      </Root>
    );
  },
);
