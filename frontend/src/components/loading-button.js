import React from "react";
import Button from "react-bootstrap/Button";

export default class LoadingButton extends React.Component {
  render () {
    const { isLoading, ...other } = this.props;

    return (
      <Button {...other}
        disabled={isLoading}
      >
        {isLoading ? 'Loading…' : this.props.children}
      </Button>
    );
  }
}