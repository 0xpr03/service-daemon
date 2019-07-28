import Alert from "react-bootstrap/Alert";
import React from "react";

export default class Error extends React.Component {
    constructor(props) {
        super(props);

        this.state = {
            hide: false,
            lastError: undefined,
        }
        this.hide = this.hide.bind(this);
    }

    hide () {
        this.setState({ hide: true });
    }

    static getDerivedStateFromProps (props, state) {
        if (props.error !== undefined) {
            if (state.hide) {
                return { hide: false, lastError: props.error }
            }
        }
        return null;
    }

    render () {
        const showError = this.props.error !== undefined;

        if (showError && !this.state.hide) {
            return (<Alert onClose={this.hide} dismissible variant="danger">{this.props.error}</Alert>)
        } else {
            return null;
        }
    }
}