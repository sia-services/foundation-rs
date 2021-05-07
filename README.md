# server
Automatized application server on Rust for Oracle

## oracle_derive
Procedural macros

## Prepare environment

***install libaio1, libssl-dev, pkg-config***

    sudo apt install libaio1 libssl-dev pkg-config -y

***prepare oracle libs***

    copy oracle instant client include to /usr/include/oracle/instantclient_19_3/

***go to instantclient/sdk/include***

    sudo mkdir -p /usr/include/oracle/instantclient_19_3/
    sudo cp * -R /usr/include/oracle/instantclient_19_3/
    sudo chmod o+r -R /usr/include/oracle

***copy oracle instant client libs to /opt/oracle/instantclient_19_3/***

    sudo mkdir -p /opt/oracle/instantclient_19_3/
    sudo cp -H -r *.so /opt/oracle/instantclient_19_3/
    sudo cp -r *.so.19.1 /opt/oracle/instantclient_19_3/

***oracle libraries***

    libclntshcore.so.19.1 
    libclntsh.so.19.1 
    libmql1.so 
    libipc1.so 
    libnnz19.so 
    libocci.so.19.1 
    liboramysql19.so 
    libocijdbc19.so 
    libociei.so

***make links***

    cd /opt/oracle/instantclient_19_3/
    ln -s libclntsh.so.19.1 libclntsh.so
    ln -s libclntshcore.so.19.1 libclntshcore.so

***add privilegies***

    sudo chmod o+r -R /opt/oracle/

***add to .bashrc:*** 

    export LD_LIBRARY_PATH="/opt/oracle/instantclient_19_3/"

***add to project run enviornment:***

    LD_LIBRARY_PATH="/opt/oracle/instantclient_19_3/"
