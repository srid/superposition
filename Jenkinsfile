def jenkinsConsoleUrl = "$env.JOB_URL" + "$env.BUILD_NUMBER"
def slackResponseArray = []

def sendSlack(color, message, jenkinsConsoleUrl, slackResponseArray) {
  def slackResponse = slackSend channel: "C04QY9BFM7Y", color: color, message: message+ "\n" + "<${jenkinsConsoleUrl} | *JOB-${env.BUILD_NUMBER}*>"
  slackResponseArray.each { item ->
    slackSend(channel: slackResponse.threadId, color: "#01F2D1" , message: "`${item}`" )
  }
}

def getRegistryHost(aws_acc_id, region) {
    return aws_acc_id + ".dkr.ecr." + region + ".amazonaws.com";
}

pipeline {
  agent { label 'hypersdk' }
  options {
    timeout(time: 20, unit: 'MINUTES')
  }
  environment {
    REGION = "ap-south-1";
    REGISTRY_HOST_SBX = getRegistryHost("701342709052", REGION);
    REGISTRY_HOST_PROD = getRegistryHost("980691203742", REGION);
    REGISTRY_HOST_NY_PROD = getRegistryHost("147728078333", REGION);
    REGISTRY_HOST_NY_SBX = getRegistryHost("463356420488", REGION);
    AUTOPILOT_HOST_SBX = "autopilot.internal.staging.mum.juspay.net";
    DOCKER_DIND_DNS = "jenkins-newton-dind.jp-internal.svc.cluster.local"
    GIT_REPO_NAME = "context-aware-config"
    CARGO_NET_GIT_FETCH_WITH_CLI=true
    SSH_AUTH_SOCK = """${sh(
        returnStdout: true,
        script: '''
          eval $(ssh-agent) > /dev/null
          ssh-add /home/jenkins/.ssh/id_rsa
          echo $SSH_AUTH_SOCK
        '''
      )}"""
  }
  stages {
    stage('Checkout') {
      steps {
        script {
	      isSkipCI = sh(script: "git log -1 --pretty=%B | grep -F -ie '[skip ci]' -e '[ci skip]'", returnStatus: true)
          if (isSkipCI == 0) {
	        env.SKIP_CI = true;
          } else {
            env.SKIP_CI = false;
          }
          env.COMMIT_HASH = sh(returnStdout: true, script: "git rev-parse --short HEAD").trim()
        }
      }
    }

    stage('Git init') {
      steps {
        script {
          sh 'rm ~/.ssh/known_hosts && ssh-keyscan ssh.bitbucket.juspay.net >>  ~/.ssh/known_hosts'
          sh 'git remote set-url origin ssh://git@ssh.bitbucket.juspay.net/picaf/${GIT_REPO_NAME}.git'
          sh 'git fetch'
          sh 'git config user.name ""Jenkins User""'
          sh 'git config user.email bitbucket.jenkins.read@juspay.in'
        }
      }
    }

    stage('Test') {
      when { expression { SKIP_CI == 'false' } }
      steps {
            script {
                def fmtResultCode = sh(script: 'cargo fmt --check', returnStatus:true)
                if (fmtResultCode != 0){
                    error("Code is not formatted properly. Please run 'cargo fmt' and commit the changes.")
                }

                def commitResultCode = sh(script: 'cog check --from-latest-tag', returnStatus:true)
                if (commitResultCode != 0) {
                    error("Commit message does not follow Conventional Commits guidelines.")
                }
            }

            sh 'make ci-test -e DOCKER_DNS=${DOCKER_DIND_DNS}'
      }
    }

    stage('Get old Version') {
      when {
        expression { SKIP_CI == 'false' }
        branch 'main'
      }
      steps {
        script {
          env.COMMIT_MSG="""${sh(returnStdout: true, script: "git log --format=format:%s -1")}"""
          env.OLD_SEMANTIC_VERSION="""${sh(
                  returnStdout: true,
                  script: '''
                  set +x;
                  cog -v get-version | tr -d "\n"
                  '''
              )}"""
        }
      }
    }

    stage('Versioning Management') {
      when {
        expression { SKIP_CI == 'false' }
        branch 'main'
      }
      steps {
        sh 'cog bump --auto --skip-ci "[skip ci]"'
      }
    }

    stage('Pushing release commit and tags') {
        when {
          expression { SKIP_CI == 'false' }
          branch 'main'
        }
        steps {
            script {
                sh "git push origin HEAD:${BRANCH_NAME}"
                sh "git push origin --tags"
            }
        }
    }

    stage('Get New Version') {
      when {
        expression { SKIP_CI == 'false' }
        branch 'main'
      }
      steps {
        script {
          env.COMMIT_HASH = """${sh(returnStdout: true, script: "git rev-parse --short HEAD")}""".trim()
          env.NEW_SEMANTIC_VERSION="""${sh(
                  returnStdout: true,
                  script: '''
                  set +x;
                  cog -v get-version | tr -d "\n"
                  '''
              )}"""
          echo "New version - ${NEW_SEMANTIC_VERSION}, Old version - ${OLD_SEMANTIC_VERSION}"
        }
      }
    }

    stage('Build Image') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
        branch 'main'
      }
      steps {
        script {
          slackResponseArray << "COMMIT BUILT : ${COMMIT_HASH}"
          slackResponseArray << "NEW_SEMANTIC_VERSION/DOCKER IMAGE TAG : ${NEW_SEMANTIC_VERSION}"
        }
        sh '''make ci-build -e \
                VERSION=${NEW_SEMANTIC_VERSION} \
                SOURCE_COMMIT=${COMMIT_HASH} \
                SSH_AUTH_SOCK=${SSH_AUTH_SOCK}
           '''
      }
    }

    stage('Push Image To Sandbox Registry') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
	    branch 'main'
      }
      steps {
	    sh '''make ci-push -e \
                VERSION=${NEW_SEMANTIC_VERSION} \
                REGION=${REGION} \
                REGISTRY_HOST=${REGISTRY_HOST_SBX}
           '''
      }
    }

    stage('Push Image To Production Registry') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
        branch 'main'
      }
      steps {
        sh '''make ci-push -e \
                VERSION=${NEW_SEMANTIC_VERSION} \
                REGION=${REGION} \
                REGISTRY_HOST=${REGISTRY_HOST_PROD}
           '''
      }
    }

    stage('Push Image To NY Sandbox Registry') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
        branch 'main'
      }
      steps {
        sh '''make ci-push -e \
                VERSION=${NEW_SEMANTIC_VERSION} \
                REGION=${REGION} \
                REGISTRY_HOST=${REGISTRY_HOST_NY_SBX}
           '''
      }
    }

    stage('Push Image To NY Production Registry') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
        branch 'main'
      }
      steps {
        sh '''make ci-push -e \
                VERSION=${NEW_SEMANTIC_VERSION} \
                REGION=${REGION} \
                REGISTRY_HOST=${REGISTRY_HOST_NY_PROD}
           '''
      }
    }

    stage('Create Integ Release Tracker') {
      when {
        expression { SKIP_CI == 'false' }
        expression { env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION }
	    branch 'main'
      }
      environment {
        CREDS = credentials('SDK_SBX_AP_KEY')
        COMMIT_MSG = sh(returnStdout: true, script: "git log --format=format:%s -1")
        CHANGE_LOG = "Commit message: ${COMMIT_MSG}";
        AUTHOR_NAME = sh(returnStdout: true, script: "git log -1 --pretty=format:'%ae'")
      }
      steps {
        sh """curl -v --location --request POST 'https://${AUTOPILOT_HOST_SBX}/release' \
                --header 'Content-Type: application/json' \
                --header 'x-api-key: ${CREDS}' \
                --data-raw '{
                      "service": ["CONTEXT_AWARE_CONFIG"],
                      "release_manager": "jenkins.jenkins",
                      "release_tag": "",
                      "new_version": "${NEW_SEMANTIC_VERSION}",
                      "docker_image" : "${NEW_SEMANTIC_VERSION}",
                      "priority" : 0,
                      "cluster" : "EKS_MUM",
                      "is_approved": 1,
                      "is_infra_approved": 1,
                      "change_log": "${CHANGE_LOG}",
                      "rollout_strategy": [
                          {
                              "rollout": 100,
                              "cooloff": 1,
                              "pods": 1
                          }
                      ],
                      "description": "${CHANGE_LOG}",
                      "product": "HYPER_SDK",
                      "mode" : "AUTO",
                      "env" : "UAT"
                }';
           """
      }
    }

    stage('Summary') {
      steps {
        script {
          echo 'Build Success'
        }
      }
    }
  }
  post {
    failure {
      script {
        if (env.BRANCH_NAME == 'main') {
          sendSlack("#AA1100", "@channel *BUILD_FAILED* ", jenkinsConsoleUrl, slackResponseArray)
        }
     }
   }
    success {
      script {
        if (env.BRANCH_NAME == 'main' && env.NEW_SEMANTIC_VERSION != env.OLD_SEMANTIC_VERSION) {
          sendSlack("#3CF700", "*BUILD SUCCESS*", jenkinsConsoleUrl, slackResponseArray)
        }
     }
    }
  }
}