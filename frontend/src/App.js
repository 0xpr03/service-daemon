import React from "react";
import {
  BrowserRouter as Router,
  Route,
  Redirect,
  NavLink
} from "react-router-dom";
import IO from "./views/io";
import Login from "./views/login";
import Users from "./views/users";
import User from "./views/user";
import Container from "react-bootstrap/Container";
import Logout from "./views/logout";
import Service from "./views/service";
import Settings from "./views/settings";
import NewUser from "./views/new_user";
import Overview from "./views/overview";
import { NavDropdown, Navbar, Nav } from "react-bootstrap";
import './App.css';
import { UserContext } from './user-context';

function About () {
  return (<Container><h2>About</h2>
    Service Controller<br />
    Copyright 2019 Aron Heinecke
      </Container>
  );
}

function Navgiation (props) {
  const isLoggedIn = props.isLoggedIn;
  if (isLoggedIn) {
    return (
      <UserContext.Consumer>
        {({ user }) => (
          <Navbar collapseOnSelect expand="md" bg="dark" variant="dark">
            <Navbar.Brand>Service Daemon</Navbar.Brand>
            <Navbar.Toggle aria-controls="responsive-navbar-nav" />
            <Navbar.Collapse id="responsive-navbar-nav">
              <Nav className="mr-auto">
                <Nav.Link as={NavLink} to="/" exact>
                  Services
              </Nav.Link>
              { user.admin && <Nav.Link as={NavLink} to="/users">Users</Nav.Link> }
              </Nav>
              <Nav style={{ "marginRight": "2em" }}>
                <NavDropdown title={user.name} id="collasible-nav-dropdown">
                  <NavDropdown.Item as={NavLink} to="/settings">Settings</NavDropdown.Item>
                  <NavDropdown.Item as={NavLink} to="/logout/">Logout</NavDropdown.Item>
                </NavDropdown>
              </Nav>
            </Navbar.Collapse>
          </Navbar>)}
      </UserContext.Consumer>
    );
  } else {
    return (null);
  }
}

function PrivateRoute ({ component: Component, isLoggedIn, ...rest }) {

  return (
    <UserContext.Consumer>
      {(context) =>
        <Route
          {...rest}
          render={props =>
            context.user !== undefined ? (
              <Component {...props} />
            ) : (
                <Redirect
                  to={{
                    pathname: "/login",
                    state: { from: props.location }
                  }}
                />
              )
          }
        />}
    </UserContext.Consumer>
  );
}

export default class App extends React.Component {
  constructor(props) {
    super(props);

    this.setUser = user => {
      console.log(user);
      this.setState({ user });
    };

    this.updateInfo = (name,email) => {
      let user = this.state.user;
      user.name = name;
      user.email = email;
      this.setState({user});
    }

    this.state = {
      user: undefined,
      setUser: this.setUser,
      updateInfo: this.updateInfo,
    };
  }

  render () {
    return (
      <UserContext.Provider value={this.state}>
        <Router>
          <div className="d-flex flex-column h-100">
            <nav>
              <Navgiation isLoggedIn={this.state.user !== undefined} />
            </nav>

            <PrivateRoute path="/" exact component={Overview} />
            <PrivateRoute path="/service/:service" exact component={Service} />
            <PrivateRoute path="/service/:service/console" component={IO} />
            <PrivateRoute path="/about/" component={About} />
            <PrivateRoute path="/settings/" component={Settings} />
            <PrivateRoute path="/users/" component={Users} />
            <PrivateRoute path="/new/user/" component={NewUser} />
            <PrivateRoute path="/user/:user" component={User} />
            <Route path="/logout/" component={Logout} />
            <Route path="/login/" component={Login} />
          </div>
        </Router>
      </UserContext.Provider>
    );
  }
}

export { App };
