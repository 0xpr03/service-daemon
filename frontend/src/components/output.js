import React from "react";
import Col from "react-bootstrap/Col";
import { LogType } from "../lib/Api";
import './output.css';

function lines (lines) {
    return lines.map((line) => parseLine(line));
}

function parseLine (line) {
    if (line[LogType.State] !== undefined) {
        return (<Col className="text-info console-col">
            {line[LogType.State]}
        </Col>);
    } else if (line[LogType.Stderr] !== undefined) {
        return (<Col className="text-danger console-col">
            {line[LogType.Stderr]}
        </Col>);
    } else if (line[LogType.Stdin] !== undefined) {
        return (<Col className="text-primary console-col">
            {line[LogType.Stdin]}
        </Col>);
    } else {
        return (<Col className="text-normal console-col">
            {line[LogType.Stdout]}
        </Col>);
    }
}

export default class Output extends React.Component {

    render () {

        return (
            <Col className="wrapperStyle">
                {lines(this.props.data)}
            </Col>
        );
    }
}