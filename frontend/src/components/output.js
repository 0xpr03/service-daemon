import React from "react";
import Col from "react-bootstrap/Col";
import { ConsoleType } from "../lib/Api";
import './output.css';

function lines (lines) {
    return lines.map((line) => parseLine(line));
}

function parseLine (line) {
    if (line[ConsoleType.State] !== undefined) {
        return (<Col className="text-info console-col">
            {line[ConsoleType.State]}
        </Col>);
    } else if (line[ConsoleType.Stderr] !== undefined) {
        return (<Col className="text-danger console-col">
            {line[ConsoleType.Stderr]}
        </Col>);
    } else if (line[ConsoleType.Stdin] !== undefined) {
        return (<Col className="text-primary console-col">
            {line[ConsoleType.Stdin]}
        </Col>);
    } else {
        return (<Col className="text-normal console-col">
            {line[ConsoleType.Stdout]}
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